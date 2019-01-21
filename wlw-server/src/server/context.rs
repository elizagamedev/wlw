use crate::hookevent::{HookEvent, HookEventC, HookResponse, PortableRECT, PosAndSizeData};
use crate::hookmanager::HookManager;
use crate::luauserdata::{self, Rect, WindowHandle};
use crate::pipeserver::{self, PipeServer};
use crossbeam_channel as xchan;
use dirs;
use rlua;
use std::error;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use winapi::shared::windef::HWND;
use winapi::shared::windef::RECT;

#[derive(Debug)]
pub enum Error {
    PipeServerInit(pipeserver::Error),
    PipeServerFail(pipeserver::Error),
    LuaScriptOpen(io::Error),
    LuaInit(rlua::Error),
    LuaCallback(rlua::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::PipeServerInit(e) => write!(f, "Error creating pipe server: {}", e),
            Error::PipeServerFail(e) => write!(f, "Error in pipe server: {}", e),
            Error::LuaScriptOpen(e) => write!(f, "Error reading Lua script: {}", e),
            Error::LuaInit(e) => write!(f, "Error initializing Lua context: {}", e),
            Error::LuaCallback(e) => write!(f, "Error running Lua callback: {}", e),
        }
    }
}

impl error::Error for Error {}

pub enum Event {
    Interrupt,
    NewRequest(pipeserver::Request<HookEventC, HookResponse>),
    PipeServerFail(pipeserver::Error),
}

pub struct Context {
    lua: rlua::Lua,
    lua_regkey: rlua::RegistryKey,
    _pipe_server: PipeServer<HookEventC, HookResponse>,
    _hook_manager: HookManager,
    event_receiver: xchan::Receiver<Event>,
}

impl Context {
    pub fn new(es: xchan::Sender<Event>, er: xchan::Receiver<Event>) -> Result<Context, Error> {
        // Load Lua script
        let mut script_path = dirs::home_dir().unwrap();
        script_path.push("wlw.lua");
        let mut script_file = File::open(script_path).map_err(Error::LuaScriptOpen)?;
        let mut script_content = String::new();
        script_file
            .read_to_string(&mut script_content)
            .map_err(Error::LuaScriptOpen)?;
        let lua = rlua::Lua::new();
        let lua_regkey = lua
            .context(move |lua_ctx| {
                let globals = lua_ctx.globals();
                globals.set("wlw", lua_ctx.create_table()?)?;
                let key = lua_ctx.create_registry_value(lua_ctx.create_table()?)?;
                lua_ctx.load(&script_content).exec()?;
                Ok(key)
            })
            .map_err(Error::LuaInit)?;

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

        Ok(Context {
            lua,
            lua_regkey,
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
                    match self.handle_hook_event(HookEvent::from(req.message)) {
                        Ok(Some(r)) => req.respond(r),
                        Ok(None) => req.acknowledge(),
                        Err(e) => return Err(e),
                    }
                }
                Event::PipeServerFail(e) => return Err(Error::PipeServerFail(e)),
            }
        }
        Ok(())
    }

    fn handle_hook_event(&mut self, event: HookEvent) -> Result<Option<HookResponse>, Error> {
        match event {
            HookEvent::CwpShowWindow { hwnd, shown } => {
                self.lua.context(|lua_ctx| {
                    let window_handle = self.get_window_handle(lua_ctx, hwnd)?;
                    Context::run_lua_callback(
                        lua_ctx,
                        "on_window_show",
                        (window_handle, shown),
                        rlua::Nil,
                    )?;
                    Ok(())
                })?;
                Ok(None)
            }
            HookEvent::CbtActivate {
                hwnd,
                caused_by_mouse,
            } => {
                self.lua.context(|lua_ctx| {
                    let window_handle = self.get_window_handle(lua_ctx, hwnd)?;
                    Context::run_lua_callback(
                        lua_ctx,
                        "on_window_activate",
                        (window_handle, caused_by_mouse),
                        rlua::Nil,
                    )?;
                    Ok(())
                })?;
                Ok(None)
            }
            HookEvent::CbtCreateWindow { hwnd, rect } => {
                let lua_rect = self.lua.context(|lua_ctx| {
                    let window_handle = self.get_window_handle(lua_ctx, hwnd)?;
                    let lua_rect = Rect::from(rect);
                    Context::run_lua_callback(
                        lua_ctx,
                        "on_window_create",
                        (window_handle, lua_rect),
                        lua_rect,
                    )
                })?;
                Ok(Some(HookResponse {
                    pos_and_size_data: PosAndSizeData {
                        rect: PortableRECT::from(RECT::from(lua_rect)),
                    },
                }))
            }
            HookEvent::CbtDestroyWindow { hwnd } => {
                self.lua
                    .context(|lua_ctx| match self.delete_window_handle(lua_ctx, hwnd) {
                        Ok(window_handle) => {
                            Context::run_lua_callback(
                                lua_ctx,
                                "on_window_destroy",
                                window_handle,
                                rlua::Nil,
                            )?;
                            Ok(())
                        }
                        Err(_) => Ok(()),
                    })?;
                Ok(None)
            }
            HookEvent::CbtMinMax { hwnd, show_command } => {
                self.lua.context(|lua_ctx| {
                    let window_handle = self.get_window_handle(lua_ctx, hwnd)?;
                    Context::run_lua_callback(
                        lua_ctx,
                        "on_window_min_max",
                        (
                            window_handle,
                            luauserdata::show_command_to_str(show_command).unwrap(),
                        ),
                        rlua::Nil,
                    )?;
                    Ok(())
                })?;
                Ok(None)
            }
            HookEvent::CbtMoveSize { hwnd, rect } => {
                let lua_rect = self.lua.context(|lua_ctx| {
                    let window_handle = self.get_window_handle(lua_ctx, hwnd)?;
                    let lua_rect = Rect::from(rect);
                    Context::run_lua_callback(
                        lua_ctx,
                        "on_window_move_resize",
                        (window_handle, lua_rect),
                        lua_rect,
                    )
                })?;
                Ok(Some(HookResponse {
                    pos_and_size_data: PosAndSizeData {
                        rect: PortableRECT::from(RECT::from(lua_rect)),
                    },
                }))
            }
        }
    }

    fn get_window_handle<'lua>(
        &self,
        lua_ctx: rlua::Context<'lua>,
        hwnd: HWND,
    ) -> Result<rlua::AnyUserData<'lua>, Error> {
        let window_table: rlua::Table = lua_ctx
            .registry_value(&self.lua_regkey)
            .map_err(Error::LuaCallback)?;
        match window_table.get(hwnd as u32) {
            Ok(handle) => Ok(handle),
            Err(_) => {
                window_table
                    .set(hwnd as u32, WindowHandle::new(hwnd))
                    .map_err(Error::LuaCallback)?;
                window_table.get(hwnd as u32).map_err(Error::LuaCallback)
            }
        }
    }

    fn delete_window_handle<'lua>(
        &self,
        lua_ctx: rlua::Context<'lua>,
        hwnd: HWND,
    ) -> Result<rlua::AnyUserData<'lua>, Error> {
        let window_table: rlua::Table = lua_ctx
            .registry_value(&self.lua_regkey)
            .map_err(Error::LuaCallback)?;
        let handle = window_table.get(hwnd as u32).map_err(Error::LuaCallback)?;
        window_table
            .set(hwnd as u32, rlua::Nil)
            .map_err(Error::LuaCallback)?;
        Ok(handle)
    }

    fn run_lua_callback<'lua, T: rlua::FromLua<'lua>>(
        lua_ctx: rlua::Context<'lua>,
        name: &str,
        args: impl rlua::ToLuaMulti<'lua>,
        default: T,
    ) -> Result<T, Error> {
        let globals = lua_ctx.globals();
        let wlw: rlua::Table = globals.get("wlw").map_err(Error::LuaCallback)?;
        if let Ok(func) = wlw.get::<_, rlua::Function>(name) {
            func.call::<_, T>(args).map_err(Error::LuaCallback)
        } else {
            Ok(default)
        }
    }
}
