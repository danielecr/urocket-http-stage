/// The front service:
/// Accepts request from tcp port:
/// 1. assign a unique request id
/// 2. accordingly to conf file:
///   * rely request to backend (executor backend)
/// 
/// Accept command from other "actors"
/// (the only actor is the executor backserv):
/// 1. match the unique request id
/// 2. send back the payload received as a response to request_id
/// 
/// Problems:
/// - the frontservice callback synchronize with backserv: it waits until the corresponding response is ready.
/// - the backserv synchronize with the frontserv: a message sent to backend is matched with a waiting frontserv's message.
/// 
/// There could be an arbiter in the middle:
///  - the arbiter provide a channel to frontserv
///  - the arbiter store the request_id associated with the channel (is it possible to store a rx in a hashmap? Maybe no, but it is possible to store rx in array?)
///  - the arbiter: 1. provide feedback to backserv, 2. send back response to frontserv, 3. dealloc/close the channel for synchronization
///  - the arbiter manage a timeout on the request, and return a standard reply
/// 

struct FrontServ {
    
}