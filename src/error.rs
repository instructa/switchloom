/// Concrete failures exposed by the Switchloom product library.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{message}")]
    InvalidInput { message: String },

    #[error("{context}: {source}")]
    Context {
        context: String,
        #[source]
        source: Box<Error>,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    TomlDecode(#[from] toml::de::Error),

    #[error(transparent)]
    TomlEncode(#[from] toml::ser::Error),
}

impl Error {
    pub(crate) fn message(message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
        }
    }

    fn context(self, context: impl Into<String>) -> Self {
        Self::Context {
            context: context.into(),
            source: Box::new(self),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) trait ResultContext<T> {
    fn with_context<C>(self, context: C) -> Result<T>
    where
        C: FnOnce() -> String;
}

impl<T, E> ResultContext<T> for std::result::Result<T, E>
where
    E: Into<Error>,
{
    fn with_context<C>(self, context: C) -> Result<T>
    where
        C: FnOnce() -> String,
    {
        self.map_err(|source| source.into().context(context()))
    }
}

#[macro_export]
macro_rules! bail {
    ($($argument:tt)*) => {
        return Err($crate::error::Error::message(format!($($argument)*)))
    };
}

#[macro_export]
macro_rules! product_error {
    ($($argument:tt)*) => {
        $crate::error::Error::message(format!($($argument)*))
    };
}
