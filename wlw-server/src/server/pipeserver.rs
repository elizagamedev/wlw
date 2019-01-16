use failure::Error;
use std::cell::RefCell;
use std::ffi::OsString;
use std::rc::Rc;
use std::thread::{self, JoinHandle};
use std::{mem, panic, ptr};
use winapi::shared::minwindef::{DWORD, FALSE, LPCVOID, LPVOID, TRUE};
use winapi::shared::ntdef::HANDLE;
use winapi::shared::winerror::{ERROR_IO_PENDING, ERROR_PIPE_CONNECTED, WAIT_TIMEOUT};
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::fileapi::{ReadFile, WriteFile};
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::ioapiset::GetOverlappedResult;
use winapi::um::minwinbase::OVERLAPPED;
use winapi::um::namedpipeapi::{ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe};
use winapi::um::synchapi::{CreateEventW, SetEvent, WaitForMultipleObjects};
use winapi::um::winbase::{
    FILE_FLAG_OVERLAPPED, INFINITE, PIPE_ACCESS_DUPLEX, PIPE_READMODE_MESSAGE, PIPE_TYPE_MESSAGE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT, WAIT_FAILED, WAIT_OBJECT_0,
};
use wlw_server::util;
use wlw_server::windowserror::WindowsError;

#[derive(Clone)]
struct Event {
    handle: HANDLE,
}

impl Event {
    fn new(initial_state: bool) -> Result<Self, WindowsError> {
        let handle = unsafe {
            CreateEventW(
                ptr::null_mut(),
                FALSE,
                if initial_state { TRUE } else { FALSE },
                ptr::null_mut(),
            )
        };
        if handle.is_null() {
            Err(WindowsError::last())
        } else {
            Ok(Event { handle })
        }
    }

    fn from(handle: HANDLE) -> Self {
        Event { handle }
    }

    fn set(&mut self) -> Result<(), WindowsError> {
        if unsafe { SetEvent(self.handle) } == FALSE {
            Err(WindowsError::last())
        } else {
            Ok(())
        }
    }

    fn free(&mut self) -> Result<(), WindowsError> {
        if unsafe { CloseHandle(self.handle) } == FALSE {
            Err(WindowsError::last())
        } else {
            Ok(())
        }
    }
}

unsafe impl Sync for Event {}
unsafe impl Send for Event {}

struct Pipe {
    handle: HANDLE,
}

impl Pipe {
    fn new(
        pipe_name: &Vec<u16>,
        output_size: usize,
        input_size: usize,
    ) -> Result<Self, WindowsError> {
        let handle = unsafe {
            CreateNamedPipeW(
                pipe_name.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                output_size as DWORD,
                input_size as DWORD,
                0,
                ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            Err(WindowsError::last())
        } else {
            Ok(Pipe { handle })
        }
    }

    fn connect(&mut self, overlap: &mut OVERLAPPED) -> Result<bool, WindowsError> {
        unsafe {
            ConnectNamedPipe(self.handle, overlap as *mut OVERLAPPED);
            let last_error = GetLastError();
            match last_error {
                ERROR_PIPE_CONNECTED => Ok(true),
                ERROR_IO_PENDING => Ok(false),
                _ => Err(WindowsError::new(last_error)),
            }
        }
    }

    fn disconnect(&mut self) -> Result<(), WindowsError> {
        if unsafe { DisconnectNamedPipe(self.handle) == FALSE } {
            Err(WindowsError::last())
        } else {
            Ok(())
        }
    }

    fn get_overlapped_result(&mut self, overlap: &mut OVERLAPPED) -> Result<usize, WindowsError> {
        let mut num_transferred: DWORD = unsafe { mem::uninitialized() };
        let success = unsafe {
            GetOverlappedResult(
                self.handle,
                overlap as *mut OVERLAPPED,
                &mut num_transferred as *mut DWORD,
                FALSE,
            )
        };
        if success == FALSE {
            Err(WindowsError::last())
        } else {
            Ok(num_transferred as usize)
        }
    }

    unsafe fn write(&mut self, data: &[u8], overlap: &mut OVERLAPPED) -> Result<(), WindowsError> {
        let result = WriteFile(
            self.handle,
            data.as_ptr() as LPCVOID,
            data.len() as DWORD,
            ptr::null_mut(),
            overlap as *mut OVERLAPPED,
        );
        if result == FALSE {
            Err(WindowsError::last())
        } else {
            Ok(())
        }
    }

    unsafe fn read(
        &mut self,
        data: &mut [u8],
        overlap: &mut OVERLAPPED,
    ) -> Result<(), WindowsError> {
        let result = ReadFile(
            self.handle,
            data.as_ptr() as LPVOID,
            data.len() as DWORD,
            ptr::null_mut(),
            overlap as *mut OVERLAPPED,
        );
        if result == FALSE {
            Err(WindowsError::last())
        } else {
            Ok(())
        }
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        if let Err(e) = self.disconnect() {
            panic!("Pipe disconnect error: {}", e);
        }
        unsafe { CloseHandle(self.handle) };
    }
}

#[derive(Debug, PartialEq)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Writing,
    Reading,
    Idle,
    FailedIo(WindowsError),
}

pub struct Connection {
    // overlap MUST be the first item in the struct
    overlap: OVERLAPPED,
    num_free_connections: Rc<RefCell<usize>>,
    on_connect: Rc<Box<dyn Fn(&mut Connection) -> Result<(), Error>>>,
    pipe: Pipe,
    state: ConnectionState,
    io_buffer: Vec<u8>,
    on_completed_write: Option<Box<dyn Fn(&mut Connection) -> Result<(), Error>>>,
    on_completed_read: Option<Box<dyn Fn(&mut Connection, Vec<u8>) -> Result<(), Error>>>,
    on_failed_io: Option<Box<dyn Fn(WindowsError) -> Result<(), Error>>>,
}

impl Connection {
    fn new(
        pipe_name: &Vec<u16>,
        event: &Event,
        num_free_connections: Rc<RefCell<usize>>,
        on_connect: Rc<Box<dyn Fn(&mut Connection) -> Result<(), Error>>>,
    ) -> Result<Box<Self>, WindowsError> {
        let mut overlap: OVERLAPPED = unsafe { mem::zeroed() };
        overlap.hEvent = event.handle;
        // TODO input and output size
        let pipe = Pipe::new(pipe_name, 300, 300)?;
        Ok(Box::new(Connection {
            overlap,
            num_free_connections,
            on_connect,
            pipe,
            state: ConnectionState::Disconnected,
            io_buffer: Vec::new(),
            on_completed_write: None,
            on_completed_read: None,
            on_failed_io: None,
        }))
    }

    pub fn write(
        &mut self,
        data: impl AsRef<[u8]>,
        on_completed_write: Option<Box<dyn Fn(&mut Connection) -> Result<(), Error>>>,
        on_failed_io: Option<Box<dyn Fn(WindowsError) -> Result<(), Error>>>,
    ) {
        trace!("Writing data to client");
        assert_eq!(self.state, ConnectionState::Idle);
        self.on_completed_write = on_completed_write;
        self.on_failed_io = on_failed_io;
        // Copy the data to write to our buffer
        self.io_buffer.clear();
        self.io_buffer.extend(data.as_ref());
        match unsafe {
            self.pipe
                .write(self.io_buffer.as_slice(), &mut self.overlap)
        } {
            Ok(_) => {
                self.state = ConnectionState::Writing;
            }
            Err(e) => {
                self.state = ConnectionState::FailedIo(e);
                Event::from(self.overlap.hEvent).set().unwrap();
            }
        }
    }

    pub fn read(
        &mut self,
        size: usize,
        on_completed_read: Option<Box<dyn Fn(&mut Connection, Vec<u8>) -> Result<(), Error>>>,
        on_failed_io: Option<Box<dyn Fn(WindowsError) -> Result<(), Error>>>,
    ) {
        trace!("Reading data from client");
        assert_eq!(self.state, ConnectionState::Idle);
        self.on_completed_read = on_completed_read;
        self.on_failed_io = on_failed_io;
        // Reserve enough memory to read the data
        self.io_buffer.resize(size, 0);
        match unsafe {
            self.pipe
                .read(self.io_buffer.as_mut_slice(), &mut self.overlap)
        } {
            Ok(_) => {
                self.state = ConnectionState::Reading;
            }
            Err(e) => {
                self.state = ConnectionState::FailedIo(e);
                Event::from(self.overlap.hEvent).set().unwrap();
            }
        }
    }

    pub fn reconnect(&mut self) -> Result<(), Error> {
        if self.state != ConnectionState::Disconnected {
            self.disconnect()?;
        }
        self.connect()?;
        Ok(())
    }

    fn on_new_connection(&mut self) -> Result<(), Error> {
        trace!("New client connected");
        self.state = ConnectionState::Idle;
        *self.num_free_connections.borrow_mut() -= 1;
        let on_connect = self.on_connect.clone();
        on_connect(self)
    }

    fn on_write_complete(&mut self) -> Result<(), Error> {
        self.state = ConnectionState::Idle;
        if let Some(cb) = self.on_completed_write.take() {
            cb(self)
        } else {
            Ok(())
        }
    }

    fn on_read_complete(&mut self) -> Result<(), Error> {
        self.state = ConnectionState::Idle;
        if let Some(cb) = self.on_completed_read.take() {
            cb(self, self.io_buffer.clone())
        } else {
            Ok(())
        }
    }

    fn on_io_error(&mut self, e: WindowsError) -> Result<(), Error> {
        self.state = ConnectionState::Idle;
        if let Some(cb) = self.on_failed_io.take() {
            cb(e)
        } else {
            Err(Error::from(e))
        }
    }

    fn connect(&mut self) -> Result<(), Error> {
        if self.state != ConnectionState::Disconnected {
            panic!("Tried to connect an already-active pipe");
        } else {
            match self.pipe.connect(&mut self.overlap) {
                Ok(true) => self.on_new_connection(),
                Ok(false) => {
                    self.state = ConnectionState::Connecting;
                    Ok(())
                }
                Err(e) => Err(Error::from(e)),
            }
        }
    }

    fn disconnect(&mut self) -> Result<(), WindowsError> {
        if self.state != ConnectionState::Disconnected {
            trace!("Disconnecting an active client connection");
            self.state = ConnectionState::Disconnected;
            *self.num_free_connections.borrow_mut() -= 1;
            self.pipe.disconnect()
        } else {
            panic!("Tried to disconnect an already-inactive pipe");
        }
    }

    fn get_overlapped_result(&mut self) -> Result<usize, WindowsError> {
        self.pipe.get_overlapped_result(&mut self.overlap)
    }
}

struct ConnectionList {
    pipe_name: Vec<u16>,
    events: Vec<Event>,
    connections: Option<Vec<Box<Connection>>>,
    num_free_connections: Rc<RefCell<usize>>,
    on_connect: Rc<Box<dyn Fn(&mut Connection) -> Result<(), Error>>>,
}

impl ConnectionList {
    fn new(
        pipe_name: Vec<u16>,
        stop_event: Event,
        num_free_connections: Rc<RefCell<usize>>,
        on_connect: Rc<Box<dyn Fn(&mut Connection) -> Result<(), Error>>>,
    ) -> ConnectionList {
        ConnectionList {
            pipe_name,
            events: vec![stop_event],
            connections: Some(Vec::new()),
            num_free_connections,
            on_connect,
        }
    }

    fn grow(&mut self, amount: usize) -> Result<(), Error> {
        trace!("Growing server connection list by {}", amount);
        *self.num_free_connections.borrow_mut() += amount;
        self.events.reserve(amount);
        self.connections.as_mut().unwrap().reserve(amount);
        for _ in 0..amount {
            let event = Event::new(false)?;
            let mut connection = Connection::new(
                &self.pipe_name,
                &event,
                self.num_free_connections.clone(),
                self.on_connect.clone(),
            )?;
            connection.connect()?;
            self.events.push(event);
            self.connections.as_mut().unwrap().push(connection);
        }
        Ok(())
    }

    fn poll(&mut self) -> Result<bool, Error> {
        let wait_result = unsafe {
            WaitForMultipleObjects(
                self.events.len() as DWORD,
                self.events.as_ptr() as *const HANDLE,
                FALSE,
                INFINITE,
            )
        };
        match wait_result {
            WAIT_FAILED => Err(Error::from(WindowsError::last())),
            WAIT_TIMEOUT => panic!("Pipe wait timed out somehow"),
            WAIT_OBJECT_0 => {
                trace!("Received stop event");
                Ok(true)
            }
            _ => {
                let index = (wait_result - WAIT_OBJECT_0 - 1) as usize;
                let conn = self.connections.as_mut().unwrap().get_mut(index).unwrap();
                trace!(
                    "Client {} has been signalled with state {:?}",
                    index,
                    conn.state
                );
                match conn.state {
                    ConnectionState::Connecting => match conn.get_overlapped_result() {
                        Ok(_) => conn.on_new_connection()?,
                        Err(e) => {
                            error!("Error connecting to client: {}", e);
                            conn.reconnect()?;
                        }
                    },
                    ConnectionState::Reading => match conn.get_overlapped_result() {
                        Ok(_) => conn.on_read_complete()?,
                        Err(e) => {
                            error!("Pipe read error: {}", e);
                            conn.on_io_error(e)?;
                            conn.reconnect()?;
                        }
                    },
                    ConnectionState::Writing => match conn.get_overlapped_result() {
                        Ok(_) => conn.on_write_complete()?,
                        Err(e) => {
                            error!("Pipe write error: {}", e);
                            conn.on_io_error(e)?;
                            conn.reconnect()?;
                        }
                    },
                    ConnectionState::FailedIo(e) => {
                        error!("Pipe I/O error: {}", e);
                        conn.on_io_error(e)?;
                        conn.reconnect()?;
                    }
                    ConnectionState::Disconnected => panic!("Disconnected state somehow signalled"),
                    ConnectionState::Idle => panic!("Idle state somehow signalled"),
                }
                Ok(false)
            }
        }
    }
}

impl Drop for ConnectionList {
    fn drop(&mut self) {
        // free connections before events
        self.connections = None;
        for event in self.events.iter_mut().skip(1) {
            // free every event but the stop event
            event.free().unwrap();
        }
    }
}

pub struct PipeServer {
    poll_thread: Option<JoinHandle<Result<(), Error>>>,
    poll_thread_stop_event: Event,
    stopped: bool,
}

impl PipeServer {
    pub fn new(
        pipe_name: impl AsRef<str>,
        on_connect: Box<dyn Fn(&mut Connection) -> Result<(), Error> + Send>,
        on_fail: Option<Box<dyn Fn() + Send>>,
    ) -> Result<Self, WindowsError> {
        trace!("Creating pipe server named \"{}\"", pipe_name.as_ref());
        let pipe_name = util::osstring_to_wstr(OsString::from(format!(
            "\\\\.\\pipe\\{}",
            pipe_name.as_ref()
        )));
        let poll_thread_stop_event = Event::new(false)?;
        let stop_event = poll_thread_stop_event.clone();
        let poll_thread = Some(thread::spawn(move || -> Result<(), Error> {
            let num_free_connections = Rc::new(RefCell::new(0));
            let mut conn_list = ConnectionList::new(
                pipe_name,
                stop_event,
                num_free_connections.clone(),
                Rc::new(on_connect),
            );

            loop {
                if *num_free_connections.borrow() == 0 {
                    if let Err(e) = conn_list.grow(16) {
                        if let Some(cb) = on_fail.as_ref() {
                            cb();
                        }
                        return Err(e);
                    }
                }
                match conn_list.poll() {
                    Ok(true) => break,
                    Ok(false) => {}
                    Err(e) => {
                        if let Some(cb) = on_fail.as_ref() {
                            cb();
                        }
                        return Err(e);
                    }
                }
            }
            Ok(())
        }));
        Ok(PipeServer {
            poll_thread,
            poll_thread_stop_event,
            stopped: false,
        })
    }

    pub fn stop(mut self) -> Result<(), Error> {
        self.stop_mut_ref()
    }

    fn stop_mut_ref(&mut self) -> Result<(), Error> {
        trace!("Stopping pipe server");
        self.stopped = true;
        self.poll_thread_stop_event.set().unwrap();
        let result = self.poll_thread.take().unwrap().join().unwrap();
        self.poll_thread_stop_event.free().unwrap();
        result
    }
}

impl Drop for PipeServer {
    fn drop(&mut self) {
        if !self.stopped {
            self.stop_mut_ref().unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flexi_logger::Logger;
    use std::str;
    use std::{thread, time};
    use winapi::um::fileapi::CreateFileW;
    use winapi::um::fileapi::WriteFile;
    use winapi::um::fileapi::OPEN_EXISTING;
    use winapi::um::namedpipeapi::SetNamedPipeHandleState;
    use winapi::um::winnt::GENERIC_READ;
    use winapi::um::winnt::GENERIC_WRITE;

    struct TestClient {
        handle: HANDLE,
    }

    impl TestClient {
        fn new(pipe_name: impl AsRef<str>) -> Result<Self, WindowsError> {
            let handle = unsafe {
                CreateFileW(
                    util::osstring_to_wstr(format!("\\\\.\\pipe\\{}", pipe_name.as_ref())).as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    0,
                    ptr::null_mut(),
                    OPEN_EXISTING,
                    0,
                    ptr::null_mut(),
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                Err(WindowsError::last())
            } else {
                let mut mode: DWORD = PIPE_READMODE_MESSAGE;
                let result = unsafe {
                    SetNamedPipeHandleState(
                        handle,
                        &mut mode as *mut DWORD,
                        ptr::null_mut(),
                        ptr::null_mut(),
                    )
                };
                if result == FALSE {
                    unsafe { CloseHandle(handle) };
                    Err(WindowsError::last())
                } else {
                    Ok(TestClient { handle })
                }
            }
        }

        fn write(&mut self, data: impl AsRef<[u8]>) -> Result<(), WindowsError> {
            let mut nbw: DWORD = unsafe { mem::uninitialized() };
            let result = unsafe {
                WriteFile(
                    self.handle,
                    data.as_ref().as_ptr() as LPCVOID,
                    data.as_ref().len() as DWORD,
                    &mut nbw as *mut DWORD,
                    ptr::null_mut(),
                )
            };
            if result == FALSE {
                Err(WindowsError::last())
            } else {
                Ok(())
            }
        }

        fn read(&mut self, size: usize) -> Result<Vec<u8>, WindowsError> {
            let mut buf: Vec<u8> = vec![0; size];
            let mut nbr: DWORD = unsafe { mem::uninitialized() };
            let result = unsafe {
                ReadFile(
                    self.handle,
                    buf.as_mut_ptr() as LPVOID,
                    buf.len() as DWORD,
                    &mut nbr as *mut DWORD,
                    ptr::null_mut(),
                )
            };
            if result == FALSE {
                Err(WindowsError::last())
            } else {
                Ok(buf)
            }
        }
    }

    impl Drop for TestClient {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.handle) };
        }
    }

    #[test]
    fn create_and_stop() {
        let ps = PipeServer::new("test_server", Box::new(|_| Ok(())), None).unwrap();
        thread::sleep(time::Duration::from_millis(1000));
        ps.stop().unwrap();
    }

    #[test]
    fn trivial_io() {
        Logger::with_str("trace").start().unwrap();
        let ps = PipeServer::new(
            "test_server",
            Box::new(|conn| {
                println!("New client!");
                conn.read(
                    7,
                    Some(Box::new(|conn, msg| {
                        println!("Got message! {}", str::from_utf8(msg.as_slice()).unwrap());
                        conn.write("RESPOND", None, None);
                        Ok(())
                    })),
                    None,
                );
                Ok(())
            }),
            None,
        )
        .unwrap();
        thread::sleep(time::Duration::from_millis(1000));
        // Test sending/receiving message
        let mut client = TestClient::new("test_server").unwrap();
        client.write("MESSAGE").unwrap();
        println!(
            "Got message: {}",
            str::from_utf8(client.read(7).unwrap().as_slice()).unwrap()
        );
        thread::sleep(time::Duration::from_millis(1000));
        ps.stop().unwrap();
    }
}
