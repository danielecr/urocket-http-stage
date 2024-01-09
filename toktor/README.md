# procedural macro for actor model

The main idea of actor model with tokio is in:
https://ryhl.io/blog/actors-with-tokio/

Currently this:

```rust
actor_handler!({ par1: &str, par2: &str} => ActorName, ActorHandler, Msg);
```

expands to:

```rust
#[derive(Clone)]
pub struct ActorHandler {
    pub sender: ::tokio::sync::mpsc::Sender<Msg>,
}
impl ActorHandler {
    pub fn new( par1: &str, par2: &str ) -> ActorHandler {
        let (sender, receiver) = ::tokio::sync::mpsc::channel(8);
        let mut actor = ActorName::new(receiver, par1: &str, par2: &str );
        ::tokio::spawn(async move { actor.run().await; });
        ActorHandler {sender}
    }
}
```

Notes:

* `ActorName` must exists
* `ActorName` must have the `new()` method
* `ActorName` must have the `async run(&self)` method
* `Msg` must exists

## TODO

Define an attribute macro for parsing this

```rust
#[toktore]
struct FeedbackInterceptor{
    actor: FeedbackInterceptor,
    message: Msg,
    #[newparam]
    servicename: &str,
    #[newparam]
    pool: MessagePool,
    ...
}
```

It would be more clean and evident.
Problem: the attribute macro type is not as easy to implement as proc_macro.

## References

- https://docs.rs/syn/latest/syn/index.html
- https://docs.rs/proc-macro2/latest/proc_macro2/index.html
- https://github.com/rust-lang/rust/issues/40090 : can not export macro_rule together
- https://github.com/dtolnay/cargo-expand
- https://github.com/serde-rs/serde/blob/master/serde_derive/src/lib.rs
- https://github.com/dtolnay/proc-macro-workshop#attribute-macro-bitfield
- https://doc.rust-lang.org/beta/reference/procedural-macros.html
- https://dev.to/dandyvica/rust-procedural-macros-step-by-step-tutorial-36n8
- https://docs.rs/quote/latest/quote/index.html
- https://github.com/resolritter/structout :: example, but using old version of `syn`
- https://www.reddit.com/r/rust/comments/n2cmvd/there_are_a_lot_of_actor_framework_projects_on/ there are too many actor model