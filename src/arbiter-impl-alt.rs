/// Unused
/// Random ideas over an Arbiter

struct ArbiterHandler {}

type AddFutureType = std::pin::Pin<Box<dyn std::future::Future<Output = Receiver<ForHttpResponse>> + Send> >;
type FulFillFutureType = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Receiver<bool>,()>> + Send>>;
trait ProxyArbiter {
    //type Response = Receiver<ForHttpResponse>;
    //type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Response> + Send> >;
    //fn add_request(&self) -> Future;
    //// Pin<Box<dyn Future<Output = Receiver<ForHttpResponse>> + Send> >
    fn add_request(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Receiver<ForHttpResponse>> + Send> >;
    fn fulfill_request(&self, request_id: &str, payload: ForHttpResponse) -> FulFillFutureType;
}


impl ArbiterHandler {
    type Response = Receiver<ForHttpResponse>;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Response> + Send> >;
    
    fn add_request(&self) -> AddFutureType {
        Box::pin(async {
        })
    }
    fn fulfill_request(&self, request_id: &str, payload: ForHttpResponse) -> FulFillFutureType {
       Box::pin(async {
       })
    }
}