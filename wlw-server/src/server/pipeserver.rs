use crossbeam_channel as xchan;
use std::cell::RefCell;
use std::ffi::{OsStr, OsString};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::{mem, ptr};
use wlw_server::windows;

#[repr(C)]
struct Event {
    handle: windows::HANDLE,
}

impl Event {
    fn new(manual_reset: bool, initial_state: bool) -> windows::Result<Self> {
        let handle = unsafe { windows::CreateEvent(ptr::null_mut(), manual_reset, initial_state) }?;
        Ok(Event { handle })
    }

    fn from(handle: windows::HANDLE) -> ManuallyDrop<Self> {
        ManuallyDrop::new(Event { handle })
    }

    fn borrow(&self) -> ManuallyDrop<Self> {
        ManuallyDrop::new(Event {
            handle: self.handle,
        })
    }

    fn set(&self) -> windows::Result<()> {
        unsafe { windows::SetEvent(self.handle) }
    }

    fn reset(&self) -> windows::Result<()> {
        unsafe { windows::ResetEvent(self.handle) }
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        unsafe { windows::CloseHandle(self.handle) }.unwrap()
    }
}

unsafe impl Sync for Event {}
unsafe impl Send for Event {}

struct Pipe {
    handle: windows::HANDLE,
}

impl Pipe {
    fn new(
        pipe_name: impl AsRef<OsStr>,
        output_size: usize,
        input_size: usize,
    ) -> windows::Result<Self> {
        let handle = unsafe {
            windows::CreateNamedPipe(
                pipe_name,
                windows::PIPE_ACCESS_DUPLEX | windows::FILE_FLAG_OVERLAPPED,
                windows::PIPE_TYPE_MESSAGE | windows::PIPE_READMODE_MESSAGE | windows::PIPE_WAIT,
                windows::PIPE_UNLIMITED_INSTANCES,
                output_size as windows::DWORD,
                input_size as windows::DWORD,
                1000,
                ptr::null_mut(),
            )?
        };
        Ok(Pipe { handle })
    }

    unsafe fn connect(
        &mut self,
        overlap: &mut windows::OVERLAPPED,
    ) -> windows::Result<windows::IoState> {
        windows::ConnectNamedPipe(self.handle, overlap as *mut _)
    }

    fn disconnect(&mut self) -> windows::Result<()> {
        unsafe { windows::DisconnectNamedPipe(self.handle) }
    }

    fn get_overlapped_result(
        &mut self,
        overlap: &mut windows::OVERLAPPED,
    ) -> windows::Result<usize> {
        unsafe {
            windows::GetOverlappedResult(self.handle, overlap as *mut _, false).map(|s| s as usize)
        }
    }

    unsafe fn write(
        &mut self,
        data: *const u8,
        size: usize,
        overlap: &mut windows::OVERLAPPED,
    ) -> windows::Result<windows::IoState> {
        windows::WriteFile(
            self.handle,
            data as windows::LPCVOID,
            size as windows::DWORD,
            ptr::null_mut(),
            overlap as *mut _,
        )
    }

    unsafe fn read(
        &mut self,
        data: *mut u8,
        size: usize,
        overlap: &mut windows::OVERLAPPED,
    ) -> windows::Result<windows::IoState> {
        windows::ReadFile(
            self.handle,
            data as windows::LPVOID,
            size as windows::DWORD,
            ptr::null_mut(),
            overlap as *mut _,
        )
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        self.disconnect().unwrap();
        unsafe { windows::CloseHandle(self.handle) }.unwrap();
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
    overlap: Box<windows::OVERLAPPED>,
    id: usize,
    num_free_connections: Rc<RefCell<usize>>,
    pipe: Pipe,
    state: ConnectionState,
    io_buffer: ReqOrRes<ReqType, ResType>,
}

impl<ReqType: Sized + Copy, ResType: Sized + Copy> Connection<ReqType, ResType> {
    fn new(
        pipe_name: impl AsRef<OsStr>,
        event: &Event,
        num_free_connections: Rc<RefCell<usize>>,
    ) -> windows::Result<Self> {
        let mut overlap: Box<windows::OVERLAPPED> = Box::new(unsafe { mem::zeroed() });
        overlap.hEvent = event.handle;
        let pipe = Pipe::new(
            pipe_name,
            mem::size_of::<ResType>(),
            mem::size_of::<ReqType>(),
        )?;
        Ok(Connection {
            overlap,
            id: 0,
            num_free_connections,
            pipe,
            state: ConnectionState::Disconnected,
            io_buffer: unsafe { mem::uninitialized() },
        })
    }

    fn write(&mut self, response: ResType) -> windows::Result<PollAction<ReqType>> {
        assert_eq!(self.state, ConnectionState::AwaitingResponse);
        unsafe {
            self.io_buffer.res = response;
            match self.pipe.write(
                &self.io_buffer as *const _ as *const u8,
                mem::size_of::<ResType>(),
                &mut *self.overlap,
            ) {
                Ok(windows::IoState::Finished) => self.on_write_complete(),
                Ok(windows::IoState::Pending) => {
                    self.state = ConnectionState::Writing;
                    Ok(PollAction::DoNothing)
                }
                Err(e) => Err(e),
            }
        }
    }

    fn connect(&mut self) -> windows::Result<PollAction<ReqType>> {
        if self.state != ConnectionState::Disconnected {
            panic!("Tried to connect an already-active pipe");
        } else {
            match unsafe { self.pipe.connect(&mut *self.overlap) } {
                Ok(windows::IoState::Finished) => self.on_new_connection(),
                Ok(windows::IoState::Pending) => {
                    self.state = ConnectionState::Connecting;
                    Ok(PollAction::DoNothing)
                }
                Err(e) => Err(e),
            }
        }
    }

    fn read(&mut self) -> windows::Result<PollAction<ReqType>> {
        match unsafe {
            self.pipe.read(
                &mut self.io_buffer as *mut _ as *mut u8,
                mem::size_of::<ReqType>(),
                &mut *self.overlap,
            )
        } {
            Ok(windows::IoState::Finished) => self.on_read_complete(),
            Ok(windows::IoState::Pending) => {
                self.state = ConnectionState::Reading;
                Ok(PollAction::DoNothing)
            }
            Err(e) => Err(e),
        }
    }

    fn on_new_connection(&mut self) -> windows::Result<PollAction<ReqType>> {
        *self.num_free_connections.borrow_mut() -= 1;
        // Begin reading from the client
        self.read()
    }

    fn on_read_complete(&mut self) -> windows::Result<PollAction<ReqType>> {
        Event::from(self.overlap.hEvent).reset()?;
        self.state = ConnectionState::AwaitingResponse;
        Ok(PollAction::DispatchRequest(unsafe { self.io_buffer.req }))
    }

    fn on_write_complete(&mut self) -> windows::Result<PollAction<ReqType>> {
        // Begin reading from client again
        self.read()
    }

    fn disconnect(&mut self) -> windows::Result<()> {
        if self.state != ConnectionState::Disconnected {
            self.id += 1;
            self.state = ConnectionState::Disconnected;
            *self.num_free_connections.borrow_mut() -= 1;
            self.pipe.disconnect()
        } else {
            panic!("Tried to disconnect an already-inactive pipe");
        }
    }

    fn reconnect(&mut self) -> windows::Result<PollAction<ReqType>> {
        if self.state != ConnectionState::Disconnected {
            self.disconnect()?;
        }
        self.connect()
    }

    fn get_overlapped_result(&mut self) -> windows::Result<usize> {
        self.pipe.get_overlapped_result(&mut *self.overlap)
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
    pipe_name: OsString,
    // Free connections before event_list
    connections: Vec<Connection<ReqType, ResType>>,
    event_list: EventList,
    response_ready_event: Arc<Event>,
    num_free_connections: Rc<RefCell<usize>>,
    on_new_request: Box<dyn Fn(Request<ReqType, ResType>) + Send>,
    incoming_response_channel: xchan::Receiver<Response<ResType>>,
    outgoing_response_channel: xchan::Sender<Response<ResType>>,
}

impl<ReqType: Sized + Copy, ResType: Sized + Copy> ConnectionList<ReqType, ResType> {
    fn new(
        pipe_name: OsString,
        stop_event: ManuallyDrop<Event>,
        num_free_connections: Rc<RefCell<usize>>,
        on_new_request: Box<dyn Fn(Request<ReqType, ResType>) + Send>,
    ) -> windows::Result<Self> {
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

    fn grow(&mut self, amount: usize) -> windows::Result<()> {
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

    fn poll(&mut self) -> windows::Result<bool> {
        let wait_result = unsafe {
            windows::WaitForMultipleObjects(
                self.event_list.events.len() as windows::DWORD,
                self.event_list.events.as_ptr() as *const windows::HANDLE,
                false,
                windows::INFINITE,
            )
        }?;
        match wait_result {
            windows::WaitResult::Timeout => panic!("Pipe wait timed out somehow"),
            windows::WaitResult::Abandoned(_) => panic!("Pipe wait abandoned somehow"),
            windows::WaitResult::Object(object) => {
                if object == 0 {
                    Ok(true)
                } else {
                    let (index, mut result) = if object == 1 {
                        let response = self.incoming_response_channel.recv().unwrap();
                        let index = response.index;
                        let conn = &mut self.connections[response.index];
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
                        let index = (object - 2) as usize;
                        let conn = &mut self.connections[index];
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
                                            panic!("Size mismatch");
                                        } else {
                                            conn.on_read_complete()
                                        }
                                    }
                                    Err(e) => Err(e),
                                },
                                ConnectionState::Writing => match conn.get_overlapped_result() {
                                    Ok(num_transferred) => {
                                        if num_transferred != mem::size_of::<ResType>() {
                                            panic!("Size mismatch");
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
                        let conn = &mut self.connections[index];
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
                            // Err(Error::DisconnectPipe(e)) => return Err(Error::DisconnectPipe(e)),
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
        on_fail: impl FnOnce(windows::Error) + Send + 'static,
    ) -> windows::Result<Self> {
        trace!("Creating pipe server named \"{}\"", pipe_name.as_ref());
        let pipe_name = OsString::from(format!("\\\\.\\pipe\\{}", pipe_name.as_ref()));
        let poll_thread_stop_event = Event::new(false, false)?;
        let stop_event = poll_thread_stop_event.borrow();
        let poll_thread = Some(thread::spawn(move || {
            let run = move || -> Result<(), windows::Error> {
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
        handle: windows::HANDLE,
        reqtype: PhantomData<ReqType>,
        restype: PhantomData<ResType>,
    }

    impl<ReqType: Sized + Copy, ResType: Sized + Copy> TestClient<ReqType, ResType> {
        fn new(pipe_name: impl AsRef<str>) -> Result<Self, windows::Error> {
            let handle = unsafe {
                windows::CreateFile(
                    format!("\\\\.\\pipe\\{}", pipe_name.as_ref()),
                    GENERIC_READ | GENERIC_WRITE,
                    0,
                    ptr::null_mut(),
                    OPEN_EXISTING,
                    0,
                    ptr::null_mut(),
                )
            }?;
            let mut mode: windows::DWORD = windows::PIPE_READMODE_MESSAGE;
            let result = unsafe {
                windows::SetNamedPipeHandleState(
                    handle,
                    &mut mode as *mut windows::DWORD,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            match result {
                Ok(_) => Ok(TestClient {
                    handle,
                    restype: PhantomData,
                    reqtype: PhantomData,
                }),
                Err(e) => {
                    unsafe { windows::CloseHandle(handle) }.ok();
                    Err(e)
                }
            }
        }

        unsafe fn write(&mut self, data: *const u8, size: usize) -> windows::Result<()> {
            let mut nbw: windows::DWORD = mem::uninitialized();
            windows::WriteFile(
                self.handle,
                data as windows::LPCVOID,
                size as windows::DWORD,
                &mut nbw as *mut windows::DWORD,
                ptr::null_mut(),
            )
        }

        unsafe fn read(&mut self, data: *mut u8, size: usize) -> windows::Result<()> {
            let mut nbr: windows::DWORD = mem::uninitialized();
            windows::ReadFile(
                self.handle,
                data as LPVOID,
                size as windows::DWORD,
                &mut nbr as *mut windows::DWORD,
                ptr::null_mut(),
            )
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
            unsafe { windows::CloseHandle(self.handle) }.unwrap();
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
