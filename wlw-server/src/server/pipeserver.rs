use crossbeam_channel as xchan;
use std::cell::RefCell;
use std::ffi::OsString;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::{error, fmt, mem, panic, ptr};
use winapi::shared::minwindef::TRUE;
use winapi::shared::minwindef::{DWORD, FALSE, LPCVOID, LPVOID};
use winapi::shared::ntdef::HANDLE;
use winapi::shared::winerror::{ERROR_IO_PENDING, ERROR_PIPE_CONNECTED, WAIT_TIMEOUT};
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::fileapi::{ReadFile, WriteFile};
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::ioapiset::GetOverlappedResult;
use winapi::um::minwinbase::OVERLAPPED;
use winapi::um::namedpipeapi::{ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe};
use winapi::um::synchapi::ResetEvent;
use winapi::um::synchapi::{CreateEventW, SetEvent, WaitForMultipleObjects};
use winapi::um::winbase::{
    FILE_FLAG_OVERLAPPED, INFINITE, PIPE_ACCESS_DUPLEX, PIPE_READMODE_MESSAGE, PIPE_TYPE_MESSAGE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT, WAIT_FAILED, WAIT_OBJECT_0,
};
use wlw_server::util;
use wlw_server::windowserror::WindowsError;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Error {
    NewEvent(WindowsError),
    SetEvent(WindowsError),
    ResetEvent(WindowsError),
    NewPipe(WindowsError),
    ConnectPipe(WindowsError),
    DisconnectPipe(WindowsError),
    GetOverlappedResult(WindowsError),
    WritePipe(WindowsError),
    ReadPipe(WindowsError),
    PollFailed(WindowsError),
    SizeMismatch,
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NewEvent(e) => write!(f, "Error creating new event: {}", e),
            Error::SetEvent(e) => write!(f, "Error setting event: {}", e),
            Error::ResetEvent(e) => write!(f, "Error resetting event: {}", e),
            Error::NewPipe(e) => write!(f, "Error creating new pipe: {}", e),
            Error::ConnectPipe(e) => write!(f, "Error connecting pipe: {}", e),
            Error::DisconnectPipe(e) => write!(f, "Error disconnecting pipe: {}", e),
            Error::GetOverlappedResult(e) => {
                write!(f, "Error getting overlapped I/O result: {}", e)
            }
            Error::WritePipe(e) => write!(f, "Error writing to pipe: {}", e),
            Error::ReadPipe(e) => write!(f, "Error reading from pipe: {}", e),
            Error::PollFailed(e) => write!(f, "Error polling pipes: {}", e),
            Error::SizeMismatch => write!(f, "Data does not match expected size"),
        }
    }
}

#[repr(C)]
struct Event {
    handle: HANDLE,
}

impl Event {
    fn new(manual_reset: bool, initial_state: bool) -> Result<Self, Error> {
        let handle = unsafe {
            CreateEventW(
                ptr::null_mut(),
                if manual_reset { TRUE } else { FALSE },
                if initial_state { TRUE } else { FALSE },
                ptr::null_mut(),
            )
        };
        if handle.is_null() {
            Err(Error::NewEvent(WindowsError::last()))
        } else {
            Ok(Event { handle })
        }
    }

    fn from(handle: HANDLE) -> ManuallyDrop<Self> {
        ManuallyDrop::new(Event { handle })
    }

    fn borrow(&self) -> ManuallyDrop<Self> {
        ManuallyDrop::new(Event {
            handle: self.handle,
        })
    }

    fn set(&self) -> Result<(), Error> {
        if unsafe { SetEvent(self.handle) } == FALSE {
            Err(Error::SetEvent(WindowsError::last()))
        } else {
            Ok(())
        }
    }

    fn reset(&self) -> Result<(), Error> {
        if unsafe { ResetEvent(self.handle) } == FALSE {
            Err(Error::ResetEvent(WindowsError::last()))
        } else {
            Ok(())
        }
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        if unsafe { CloseHandle(self.handle) } == FALSE {
            panic!("Event CloseHandle failed: {}", WindowsError::last());
        }
    }
}

unsafe impl Sync for Event {}
unsafe impl Send for Event {}

struct Pipe {
    handle: HANDLE,
}

impl Pipe {
    fn new(pipe_name: &Vec<u16>, output_size: usize, input_size: usize) -> Result<Self, Error> {
        let handle = unsafe {
            CreateNamedPipeW(
                pipe_name.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                output_size as DWORD,
                input_size as DWORD,
                1000,
                ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            Err(Error::NewPipe(WindowsError::last()))
        } else {
            Ok(Pipe { handle })
        }
    }

    unsafe fn connect(&mut self, overlap: &mut OVERLAPPED) -> Result<bool, Error> {
        ConnectNamedPipe(self.handle, overlap as *mut OVERLAPPED);
        let last_error = GetLastError();
        match last_error {
            ERROR_PIPE_CONNECTED => Ok(true),
            ERROR_IO_PENDING => Ok(false),
            _ => Err(Error::ConnectPipe(WindowsError::new(last_error))),
        }
    }

    fn disconnect(&mut self) -> Result<(), Error> {
        if unsafe { DisconnectNamedPipe(self.handle) == FALSE } {
            Err(Error::DisconnectPipe(WindowsError::last()))
        } else {
            Ok(())
        }
    }

    fn get_overlapped_result(&mut self, overlap: &mut OVERLAPPED) -> Result<usize, Error> {
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
            Err(Error::GetOverlappedResult(WindowsError::last()))
        } else {
            Ok(num_transferred as usize)
        }
    }

    unsafe fn write(
        &mut self,
        data: *const u8,
        size: usize,
        overlap: &mut OVERLAPPED,
    ) -> Result<bool, Error> {
        let mut num_written: DWORD = mem::uninitialized();
        let result = WriteFile(
            self.handle,
            data as LPCVOID,
            size as DWORD,
            &mut num_written as *mut DWORD,
            overlap as *mut OVERLAPPED,
        );
        if result != FALSE {
            if (num_written as usize) == size {
                Ok(true)
            } else {
                unreachable!();
            }
        } else {
            let last_error = GetLastError();
            if last_error == ERROR_IO_PENDING {
                Ok(false)
            } else {
                Err(Error::ReadPipe(WindowsError::new(last_error)))
            }
        }
    }

    unsafe fn read(
        &mut self,
        data: *mut u8,
        size: usize,
        overlap: &mut OVERLAPPED,
    ) -> Result<bool, Error> {
        let mut num_read: DWORD = mem::uninitialized();
        let result = ReadFile(
            self.handle,
            data as LPVOID,
            size as DWORD,
            &mut num_read as *mut DWORD,
            overlap as *mut OVERLAPPED,
        );
        if result != FALSE {
            if (num_read as usize) == size {
                Ok(true)
            } else {
                unreachable!();
            }
        } else {
            let last_error = GetLastError();
            if last_error == ERROR_IO_PENDING {
                Ok(false)
            } else {
                Err(Error::ReadPipe(WindowsError::new(last_error)))
            }
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

pub struct Request<ReqType: Sized + Copy, ResType: Sized + Copy> {
    pub message: ReqType,
    index: usize,
    id: usize,
    event: Arc<Event>,
    channel: xchan::Sender<Response<ResType>>,
}

struct Response<ResType: Sized + Copy> {
    message: Option<ResType>,
    index: usize,
    id: usize,
}

impl<ReqType: Sized + Copy, ResType: Sized + Copy> Request<ReqType, ResType> {
    pub fn respond(self, message: ResType) {
        self.event.set().unwrap();
        self.channel
            .send(Response {
                message: Some(message),
                index: self.index,
                id: self.id,
            })
            .unwrap();
    }

    pub fn acknowledge(self) {
        self.event.set().unwrap();
        self.channel
            .send(Response {
                message: None,
                index: self.index,
                id: self.id,
            })
            .unwrap();
    }
}

#[derive(Debug, PartialEq)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Writing,
    Reading,
    AwaitingResponse,
}

#[derive(PartialEq)]
enum PollAction<ReqType: Sized + Copy> {
    DoNothing,
    DispatchRequest(ReqType),
}

#[repr(C)]
union ReqOrRes<ReqType: Sized + Copy, ResType: Sized + Copy> {
    req: ReqType,
    res: ResType,
}

struct Connection<ReqType: Sized + Copy, ResType: Sized + Copy> {
    overlap: OVERLAPPED,
    id: usize,
    num_free_connections: Rc<RefCell<usize>>,
    pipe: Pipe,
    state: ConnectionState,
    io_buffer: ReqOrRes<ReqType, ResType>,
}

impl<ReqType: Sized + Copy, ResType: Sized + Copy> Connection<ReqType, ResType> {
    fn new(
        pipe_name: &Vec<u16>,
        event: &Event,
        num_free_connections: Rc<RefCell<usize>>,
    ) -> Result<Box<Self>, Error> {
        let mut overlap: OVERLAPPED = unsafe { mem::zeroed() };
        overlap.hEvent = event.handle;
        let pipe = Pipe::new(
            pipe_name,
            mem::size_of::<ResType>(),
            mem::size_of::<ReqType>(),
        )?;
        Ok(Box::new(Connection {
            overlap,
            id: 0,
            num_free_connections,
            pipe,
            state: ConnectionState::Disconnected,
            io_buffer: unsafe { mem::uninitialized() },
        }))
    }

    fn write(&mut self, response: ResType) -> Result<PollAction<ReqType>, Error> {
        assert_eq!(self.state, ConnectionState::AwaitingResponse);
        unsafe {
            self.io_buffer.res = response;
            match self.pipe.write(
                &self.io_buffer as *const _ as *const u8,
                mem::size_of::<ResType>(),
                &mut self.overlap,
            ) {
                Ok(true) => self.on_write_complete(),
                Ok(false) => {
                    self.state = ConnectionState::Writing;
                    Ok(PollAction::DoNothing)
                }
                Err(e) => Err(e),
            }
        }
    }

    fn connect(&mut self) -> Result<PollAction<ReqType>, Error> {
        if self.state != ConnectionState::Disconnected {
            panic!("Tried to connect an already-active pipe");
        } else {
            match unsafe { self.pipe.connect(&mut self.overlap) } {
                Ok(true) => self.on_new_connection(),
                Ok(false) => {
                    self.state = ConnectionState::Connecting;
                    Ok(PollAction::DoNothing)
                }
                Err(e) => Err(e),
            }
        }
    }

    fn read(&mut self) -> Result<PollAction<ReqType>, Error> {
        match unsafe {
            self.pipe.read(
                &mut self.io_buffer as *mut _ as *mut u8,
                mem::size_of::<ReqType>(),
                &mut self.overlap,
            )
        } {
            Ok(true) => self.on_read_complete(),
            Ok(false) => {
                self.state = ConnectionState::Reading;
                Ok(PollAction::DoNothing)
            }
            Err(e) => Err(e),
        }
    }

    fn on_new_connection(&mut self) -> Result<PollAction<ReqType>, Error> {
        *self.num_free_connections.borrow_mut() -= 1;
        // Begin reading from the client
        self.read()
    }

    fn on_read_complete(&mut self) -> Result<PollAction<ReqType>, Error> {
        Event::from(self.overlap.hEvent).reset()?;
        self.state = ConnectionState::AwaitingResponse;
        Ok(PollAction::DispatchRequest(unsafe { self.io_buffer.req }))
    }

    fn on_write_complete(&mut self) -> Result<PollAction<ReqType>, Error> {
        // Begin reading from client again
        self.read()
    }

    fn disconnect(&mut self) -> Result<(), Error> {
        if self.state != ConnectionState::Disconnected {
            self.id += 1;
            self.state = ConnectionState::Disconnected;
            *self.num_free_connections.borrow_mut() -= 1;
            self.pipe.disconnect()
        } else {
            panic!("Tried to disconnect an already-inactive pipe");
        }
    }

    fn reconnect(&mut self) -> Result<PollAction<ReqType>, Error> {
        if self.state != ConnectionState::Disconnected {
            self.disconnect()?;
        }
        self.connect()
    }

    fn get_overlapped_result(&mut self) -> Result<usize, Error> {
        self.pipe.get_overlapped_result(&mut self.overlap)
    }
}

struct EventList {
    events: Vec<ManuallyDrop<Event>>,
}

impl EventList {
    fn new(stop_event: ManuallyDrop<Event>, response_ready_event: ManuallyDrop<Event>) -> Self {
        EventList {
            events: vec![stop_event, response_ready_event],
        }
    }
}

impl Drop for EventList {
    fn drop(&mut self) {
        for event in self.events.iter_mut().skip(2) {
            // free every event but the stop and response events
            unsafe { ManuallyDrop::drop(event) };
        }
    }
}

struct ConnectionList<ReqType: Sized + Copy, ResType: Sized + Copy> {
    pipe_name: Vec<u16>,
    // Free connections before event_list
    connections: Vec<Box<Connection<ReqType, ResType>>>,
    event_list: EventList,
    response_ready_event: Arc<Event>,
    num_free_connections: Rc<RefCell<usize>>,
    on_new_request: Box<dyn Fn(Request<ReqType, ResType>) + Send>,
    incoming_response_channel: xchan::Receiver<Response<ResType>>,
    outgoing_response_channel: xchan::Sender<Response<ResType>>,
}

impl<ReqType: Sized + Copy, ResType: Sized + Copy> ConnectionList<ReqType, ResType> {
    fn new(
        pipe_name: Vec<u16>,
        stop_event: ManuallyDrop<Event>,
        num_free_connections: Rc<RefCell<usize>>,
        on_new_request: Box<dyn Fn(Request<ReqType, ResType>) + Send>,
    ) -> Result<Self, Error> {
        let (outgoing_response_channel, incoming_response_channel) = xchan::unbounded();
        let response_ready_event = Event::new(false, false)?;
        let response_ready_event_borrow = response_ready_event.borrow();
        Ok(ConnectionList {
            pipe_name,
            connections: Vec::new(),
            response_ready_event: Arc::new(response_ready_event),
            event_list: EventList::new(stop_event, response_ready_event_borrow),
            num_free_connections,
            on_new_request,
            incoming_response_channel,
            outgoing_response_channel,
        })
    }

    fn grow(&mut self, amount: usize) -> Result<(), Error> {
        *self.num_free_connections.borrow_mut() += amount;
        self.event_list.events.reserve(amount);
        self.connections.reserve(amount);
        for _ in 0..amount {
            let event = ManuallyDrop::new(Event::new(false, false)?);
            let mut connection =
                Connection::new(&self.pipe_name, &event, self.num_free_connections.clone())?;
            connection.connect()?;
            self.event_list.events.push(event);
            self.connections.push(connection);
        }
        Ok(())
    }

    fn poll(&mut self) -> Result<bool, Error> {
        let wait_result = unsafe {
            WaitForMultipleObjects(
                self.event_list.events.len() as DWORD,
                self.event_list.events.as_ptr() as *const HANDLE,
                FALSE,
                INFINITE,
            )
        };
        match wait_result {
            WAIT_FAILED => Err(Error::PollFailed(WindowsError::last())),
            WAIT_TIMEOUT => panic!("Pipe wait timed out somehow"),
            WAIT_OBJECT_0 => Ok(true),
            _ => {
                let (index, mut result) = if wait_result == WAIT_OBJECT_0 + 1 {
                    let response = self.incoming_response_channel.recv().unwrap();
                    let index = response.index;
                    let conn = self.connections.get_mut(response.index).unwrap();
                    if conn.id == response.id {
                        if conn.state == ConnectionState::AwaitingResponse {
                            match response.message {
                                Some(message) => (index, conn.write(message)),
                                None => (index, conn.read()),
                            }
                        } else {
                            unreachable!();
                        }
                    } else {
                        (index, Ok(PollAction::DoNothing))
                    }
                } else {
                    let index = (wait_result - WAIT_OBJECT_0 - 2) as usize;
                    let conn = self.connections.get_mut(index).unwrap();
                    (
                        index,
                        match conn.state {
                            ConnectionState::Connecting => match conn.get_overlapped_result() {
                                Ok(_) => conn.on_new_connection(),
                                Err(e) => Err(e),
                            },
                            ConnectionState::Reading => match conn.get_overlapped_result() {
                                Ok(num_transferred) => {
                                    if num_transferred != mem::size_of::<ReqType>() {
                                        Err(Error::SizeMismatch)
                                    } else {
                                        conn.on_read_complete()
                                    }
                                }
                                Err(e) => Err(e),
                            },
                            ConnectionState::Writing => match conn.get_overlapped_result() {
                                Ok(num_transferred) => {
                                    if num_transferred != mem::size_of::<ResType>() {
                                        Err(Error::SizeMismatch)
                                    } else {
                                        conn.on_write_complete()
                                    }
                                }
                                Err(e) => Err(e),
                            },
                            ConnectionState::Disconnected => {
                                panic!("Disconnected state somehow signalled")
                            }
                            ConnectionState::AwaitingResponse => {
                                panic!("Await-response state somehow signalled")
                            }
                        },
                    )
                };

                loop {
                    let conn = self.connections.get_mut(index).unwrap();
                    result = match result {
                        Ok(PollAction::DoNothing) => break Ok(false),
                        Ok(PollAction::DispatchRequest(message)) => {
                            let request = Request {
                                index,
                                id: conn.id,
                                message,
                                event: self.response_ready_event.clone(),
                                channel: self.outgoing_response_channel.clone(),
                            };
                            (self.on_new_request)(request);
                            Ok(PollAction::DoNothing)
                        }
                        Err(Error::DisconnectPipe(e)) => return Err(Error::DisconnectPipe(e)),
                        Err(e) => {
                            error!("Pipe connection problem: {}", e);
                            conn.reconnect()
                        }
                    };
                }
            }
        }
    }
}

pub struct PipeServer<ReqType: Sized + Copy, ResType: Sized + Copy> {
    poll_thread: Option<JoinHandle<()>>,
    poll_thread_stop_event: Event,
    reqtype: PhantomData<ReqType>,
    restype: PhantomData<ResType>,
}

impl<ReqType: Sized + Copy, ResType: Sized + Copy> PipeServer<ReqType, ResType> {
    pub fn new(
        pipe_name: impl AsRef<str>,
        on_new_request: impl Fn(Request<ReqType, ResType>) + Send + 'static,
        on_fail: impl FnOnce(Error) + Send + 'static,
    ) -> Result<Self, Error> {
        trace!("Creating pipe server named \"{}\"", pipe_name.as_ref());
        let pipe_name = util::osstring_to_wstr(OsString::from(format!(
            "\\\\.\\pipe\\{}",
            pipe_name.as_ref()
        )));
        let poll_thread_stop_event = Event::new(false, false)?;
        let stop_event = poll_thread_stop_event.borrow();
        let poll_thread = Some(thread::spawn(move || {
            let run = move || -> Result<(), Error> {
                let num_free_connections = Rc::new(RefCell::new(0));
                let mut conn_list = ConnectionList::new(
                    pipe_name,
                    stop_event,
                    num_free_connections.clone(),
                    Box::new(on_new_request),
                )?;

                loop {
                    if *num_free_connections.borrow() == 0 {
                        conn_list.grow(16)?;
                    }
                    match conn_list.poll() {
                        Ok(true) => break,
                        Ok(false) => {}
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
                Ok(())
            };
            if let Err(e) = run() {
                on_fail(e);
            }
        }));
        Ok(PipeServer {
            poll_thread,
            poll_thread_stop_event,
            reqtype: PhantomData,
            restype: PhantomData,
        })
    }
}

impl<ReqType: Sized + Copy, ResType: Sized + Copy> Drop for PipeServer<ReqType, ResType> {
    fn drop(&mut self) {
        self.poll_thread_stop_event.set().unwrap();
        self.poll_thread.take().unwrap().join().unwrap();
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

    struct TestClient<ReqType: Sized + Copy, ResType: Sized + Copy> {
        handle: HANDLE,
        reqtype: PhantomData<ReqType>,
        restype: PhantomData<ResType>,
    }

    impl<ReqType: Sized + Copy, ResType: Sized + Copy> TestClient<ReqType, ResType> {
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
                    Ok(TestClient {
                        handle,
                        restype: PhantomData,
                        reqtype: PhantomData,
                    })
                }
            }
        }

        unsafe fn write(&mut self, data: *const u8, size: usize) -> Result<(), WindowsError> {
            let mut nbw: DWORD = mem::uninitialized();
            let result = WriteFile(
                self.handle,
                data as LPCVOID,
                size as DWORD,
                &mut nbw as *mut DWORD,
                ptr::null_mut(),
            );
            if result == FALSE {
                Err(WindowsError::last())
            } else {
                Ok(())
            }
        }

        unsafe fn read(&mut self, data: *mut u8, size: usize) -> Result<(), WindowsError> {
            let mut nbr: DWORD = mem::uninitialized();
            let result = ReadFile(
                self.handle,
                data as LPVOID,
                size as DWORD,
                &mut nbr as *mut DWORD,
                ptr::null_mut(),
            );

            if result == FALSE {
                Err(WindowsError::last())
            } else {
                Ok(())
            }
        }

        fn request(&mut self, req: ReqType) -> ResType {
            unsafe {
                self.write(&req as *const _ as *const u8, mem::size_of::<ReqType>())
                    .unwrap()
            };
            let mut res = unsafe { mem::uninitialized() };
            unsafe {
                self.read(&mut res as *mut _ as *mut u8, mem::size_of::<ResType>())
                    .unwrap()
            };
            res
        }
    }

    impl<ReqType: Sized + Copy, ResType: Sized + Copy> Drop for TestClient<ReqType, ResType> {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.handle) };
        }
    }

    #[test]
    fn create_and_stop() {
        let _ps = PipeServer::new(
            "wlw_test_create_and_stop",
            |_: Request<usize, usize>| {},
            |_| {},
        )
        .unwrap();
        thread::sleep(time::Duration::from_millis(1000));
    }

    #[test]
    fn trivial_reqres() {
        Logger::with_str("trace").start().unwrap();
        let _ps = PipeServer::new(
            "wlw_test_trivial_reqres",
            |request: Request<[u8; 4], [u8; 4]>| {
                trace!("GOT REQUEST: {:?}", request.message);
                let response = [0, 1, 2, 3];
                request.respond(response);
            },
            |_| {
                error!("Server broke :(");
            },
        )
        .unwrap();
        thread::sleep(time::Duration::from_millis(1000));

        // Test sending/receiving message
        let mut client: TestClient<[u8; 4], [u8; 4]> =
            TestClient::new("wlw_test_trivial_reqres").unwrap();
        trace!("GOT RESPONSE: {:?}", client.request([3, 2, 1, 0]));
        thread::sleep(time::Duration::from_millis(1000));
    }
}
