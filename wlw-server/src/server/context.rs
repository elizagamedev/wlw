use crate::hookevent::{HookEvent, HookEventC};
use crate::hookmanager::HookManager;
use crate::pipeserver::{self, PipeServer};
use crossbeam_channel as xchan;
// use rlua::{Function, Lua, MetaMethod, Result, UserData, UserDataMethods, Variadic};
use rlua;
use std::error;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    PipeServerInit(pipeserver::Error),
    PipeServerFail(pipeserver::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::PipeServerInit(e) => write!(f, "Error creating pipe server: {}", e),
            Error::PipeServerFail(e) => write!(f, "Error in pipe server: {}", e),
        }
    }
}

impl error::Error for Error {}

pub enum Event {
    Interrupt,
    NewRequest(pipeserver::Request<HookEventC, u32>),
    PipeServerFail(pipeserver::Error),
}

pub struct Context {
    _lua: rlua::Lua,
    _pipe_server: PipeServer<HookEventC, u32>,
    _hook_manager: HookManager,
    event_receiver: xchan::Receiver<Event>,
}

impl Context {
    pub fn new(es: xchan::Sender<Event>, er: xchan::Receiver<Event>) -> Result<Context, Error> {
        let pipe_name = format!("wlw_server_{}", std::process::id());

        let pipe_server_req_es = es.clone();
        let pipe_server_fail_es = es.clone();
        trace!("Creating pipe server");
        let _pipe_server = PipeServer::new(
            pipe_name,
            move |req| pipe_server_req_es.send(Event::NewRequest(req)).unwrap(),
            move |e| pipe_server_fail_es.send(Event::PipeServerFail(e)).unwrap(),
        )
        .map_err(Error::PipeServerInit)?;
        trace!("Creating hook manager");
        let _hook_manager = HookManager::new();
        trace!("Creating Lua context");
        let _lua = rlua::Lua::new();

        Ok(Context {
            _lua,
            _pipe_server,
            _hook_manager,
            event_receiver: er,
        })
    }

    pub fn run(&mut self) -> Result<(), Error> {
        trace!("Entering event loop");
        loop {
            let event = self.event_receiver.recv().unwrap();
            match event {
                Event::Interrupt => break,
                Event::NewRequest(req) => {
                    trace!("New request!");
                    req.respond(1234);
                }
                Event::PipeServerFail(e) => return Err(Error::PipeServerFail(e)),
            }
        }
        Ok(())
    }
}
