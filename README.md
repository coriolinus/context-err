# `context-err`: contextual typed error enums made easy

## Motivation

[`thiserror`] is great. It gives us a very simple way to create custom error enums which implement the [`Error`][std::error::Error] trait, including propagation of error sources, so we can derive a chain of increasingly-detailed errors representing the failure from the highest abstraction level to the lowest.

Unfortunately, it doesn't make it easy to add context to the errors. People very often end up with error-wrapping antipatterns in their code:

### Antipattern 1: Omit Relevant Information

Definition:

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("something went wrong with the http request")]
    Reqwest(#[from] reqwest::Error),
    #[error("an io error occurred")]
    Io(#[from] std::io::Error),
}
```

Usage:

```rust
let client = Client::new()?;
let response = client.get(&url)
    .send()?
    .error_for_status()?;
let mut file = std::fs::create(&path)?;
response.copy_to(&mut file)?;
```

This pattern makes things very simple and ergonomic, but also difficult to debug. If an `Error::Reqwest` bubbles up, where did it come from? We can't know; there isn't enough information available.

### Antipattern 2: Map All The Things

Error definition:

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("building client")]
    ClientBuilder(#[source] reqwest::Error),
    #[error("requesting file")]
    RequestingInput(#[source] reqwest::Error),
    #[error("status indicates failure")]
    ResponseStatus(#[source] reqwest::Error),
    #[error("creating writeable file")
    CreateFile(#[source] std::io::Error),
    #[error("downloading into local file")]
    Downloading(#[source] reqwest::Error),
}
```

Usage:

```rust
let client = Client::new().map_err(Error::ClientBuilder)?;
let mut response = client.get(&url)
    .send()
    .map_err(Error::RequestingInput)?
    .error_for_status()
    .map_err(Error::ResponseStatus)?;
let file = std::fs::create(&path).map_err(Error::CreateFile)?;
let mut buffer = BufWriter::new(file);
response.copy_to(&mut buffer).map_err(Error::Downloading)?;
```

This pattern gives much more information about what precisely has gone wrong when an error occurs, but it is verbose and repetitive. Worse, the actual error messages are defined far from where they are used.

## We Can Do Better

What we really want is a `.context` method, reminiscent of what `Anyhow` does, but in a proper error enum. We can have that!

```rust
#[derive(Debug)]
#[derive_context_err]
pub enum Error {
    #[error(contextual)]
    Reqwest(reqwest::Error),
    #[error(contextual)]
    Io(std::io::Error),
}
```

Usage:

```rust
let client = Client::new().context("building client")?;
let response = client.get(&url)
    .send()
    .context("requesting file")?
    .error_for_status()
    .context("status indicates failure")?;
let mut file = std::fs::create(&path).context("creating writeable file")?;
response.copy_to(&mut file).context("downloading into local file")?;
```

Under the hood, this expands into something like this:

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{1}")]
    Reqwest(#[source] reqwest::Error, String),
    #[error("{1}")]
    Io(#[source] std::io::Error, String),
}

pub trait ContextErr {
    type Ok;
    fn context<S>(self, s: S) -> Result<Self::Ok, Error>
    where
        S: ToString;
}

impl<T> ContextErr for Result<T, reqwest::Error> {
    type Ok = T;
    fn context<S>(self, s: S) -> Result<T, Error>
    where
        S: ToString,
    {
        self.map_err(|inner| Error::Reqwest(inner, s.to_string()))
    }
}

impl<T> ContextErr for Result<T, std::io::Error> {
    type Ok = T;
    fn context<S>(self, s: S) -> Result<T, Error>
    where
        S: ToString,
    {
        self.map_err(|inner| Error::Io(inner, s.to_string()))
    }
}
```

Each contextual error type gets a `.context` method which converts it to our own `Error` type. As long as the `ContextErr` trait is in scope, the easiest way to handle an error coming from an upstream source is also the right way: to wrap it up with some context.

Note that all error variants which are not marked as `#[error(contextual)]` get passed through unchanged to `thiserror`'s derive macro, so it's perfectly fine to mix and match error variants.

Note also that `derive_context_error` is an attribute macro, not a standard derive macro. This is because it needs to access and edit the definition of the item that it is attached to.

### Multiple Error Types

Sometimes it is desirable to define more than a single error type per module. However, that raises obvious problems if each error type declares its own `pub trait ContextErr`. To solve this, it is possible to specify the trait name:

```rust
#[derive(Debug)]
#[derive_context_err(trait = "ContextErr1")]
#[error(contextual)]
pub struct Error1(std::io::Error);

#[derive(Debug)]
#[derive_context_err(trait = "ContextErr2")]
#[error(contextual)]
pub struct Error2(std::io::Error);
```

This expands into something like:

```rust
#[derive(Debug, thiserror::Error)]
#[error("{1}")]
pub struct Error1(#[source] std::io::Error);

pub trait ContextErr1 {
    type Ok;
    fn context<S>(self, s: S) -> Result<Self::Ok, Error1>
    where
        S: ToString;
}

impl<T> ContextErr1 for Result<T, std::io::Error> {
    type Ok = T;
    fn context<S>(self, s: S) -> Result<T, Error1>
    where
        S: ToString,
    {
        self.map_err(|inner| Error1(inner, s.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{1}")]
pub struct Error2(#[source] std::io::Error);

pub trait ContextErr2 {
    type Ok;
    fn context<S>(self, s: S) -> Result<Self::Ok, Error2>
    where
        S: ToString;
}

impl<T> ContextErr2 for Result<T, std::io::Error> {
    type Ok = T;
    fn context<S>(self, s: S) -> Result<T, Error2>
    where
        S: ToString,
    {
        self.map_err(|inner| Error2(inner, s.to_string()))
    }
}
```

Note that in this case, you will then need to use [explicit syntax](https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#fully-qualified-syntax-for-disambiguation-calling-methods-with-the-same-name) to add context to a `std::io::Error`, because Rust can no longer infer which error variant is desired. However, as long as a particular wrapped error type only appears in a single custom type, then Rust can infer which `.context` method is desired.

### Additional Context

Sometimes it's desirable to add additional context to your error wrapper, beyond a simple string. Unfortunately `context-err` does not and will not provide for this case. This is becasue the required functions would not be compatible with the generated `ContextErr` trait. In this case, your best bet is to map your own errors:

```rust
#[derive(Debug)]
#[derive_context_error]
pub enum Error {
    #[error(contextual)]
    Foo(foo::Error),
    #[error("{description}")]
    WantsContext {
        #[source] inner: wants_contextual_errors::Error,
        context: Context,
        description: String,
    },
}
```

```rust
let context = Context::new(); // etc
wants_contextual_errors::bar().map_err(|inner| Error::WantsContext {
    inner,
    context,
    description: "doing bar".into(),
})?;
```
