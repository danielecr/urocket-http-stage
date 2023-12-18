pub mod urconfig;
pub mod serviceconf;
pub mod cmdlineparser;
pub mod arbiter;
pub mod frontserv;
pub mod backserv;
pub mod requestsvisor;
pub mod restmessage;
pub mod processcontroller;
pub mod procenv;

#[macro_export]
macro_rules! toktor_send {
    ($actorname:ident,$message:ident) => {
        $actorname.sender.send($message)
    };
}

#[macro_export]
macro_rules! toktor_new {
    ($actorhand:ident, $($x:expr),* ) => {
        {
            $actorhand::new($($x),*)
        }
    };
    ($actorhand:ident) => {
        {
            $actorhand::new()
        }
    };
}
