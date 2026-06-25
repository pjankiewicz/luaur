//! Error type and `Result` alias, mirroring mlua's [`Error`] / [`Result`].
//!
//! We expose the common, developer-facing subset of mlua's `Error` variants
//! (`RuntimeError`, `SyntaxError`, the two conversion errors, etc.) plus the
//! `Error::external` / `Error::runtime` constructors. Variants specific to
//! features we have not implemented (async, serde, scopes, registry keys) are
//! intentionally omitted.

use std::error::Error as StdError;
use std::fmt;
use std::sync::Arc;

// `Result` is defined below; `Arc<Error>` is used by `Error::CallbackError`.

/// A boxed standard error, used by [`Error::ExternalError`].
type DynStdError = dyn StdError + Send + Sync;

/// The result type used throughout `luaur-rt`, mirroring [`mlua::Result`].
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when interacting with the Lua engine.
///
/// The variant set mirrors the commonly used part of mlua's `Error`. It is
/// marked `#[non_exhaustive]` (like mlua's) so new variants can be added
/// without a breaking change.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Error {
    /// A Lua syntax (compile/parse) error.
    SyntaxError {
        /// The human-readable message produced by the compiler.
        message: String,
        /// Whether the input looked like it was merely incomplete (e.g. an
        /// unterminated block). Always `false` for now; reserved for REPL use.
        incomplete_input: bool,
    },
    /// A Lua runtime error (`error(..)`, a failed `assert`, a type error, or a
    /// Rust callback returning `Err`).
    RuntimeError(String),
    /// A memory allocation error reported by the VM.
    MemoryError(String),
    /// A value could not be converted **from** a Lua value into the requested
    /// Rust type.
    FromLuaConversionError {
        /// The Lua type name of the source value.
        from: &'static str,
        /// The name of the target Rust type.
        to: String,
        /// Optional extra detail.
        message: Option<String>,
    },
    /// A Rust value could not be converted **into** a Lua value.
    ToLuaConversionError {
        /// The name of the source Rust type.
        from: &'static str,
        /// The Lua type name being targeted.
        to: &'static str,
        /// Optional extra detail.
        message: Option<String>,
    },
    /// A `UserData` value was accessed as the wrong concrete type.
    UserDataTypeMismatch,
    /// A `UserData` value was used after it had been destructed (dropped).
    UserDataDestructed,
    /// Either a callback or a userdata method was called, but the callback or
    /// userdata had been destructed.
    ///
    /// This happens when a function/userdata created via [`Lua::scope`] is used
    /// after the scope has ended (so the scope has already dropped the boxed
    /// closure / invalidated the Lua object). Mirrors
    /// `mlua::Error::CallbackDestructed`.
    ///
    /// [`Lua::scope`]: crate::Lua::scope
    CallbackDestructed,
    /// A Rust callback returned `Err`, which was raised as a Lua error and then
    /// caught at a protected-call boundary (e.g. [`Function::call`]). The
    /// original error is preserved in `cause`. Mirrors
    /// `mlua::Error::CallbackError`.
    ///
    /// luaur-rt only produces this variant for callback errors that carry
    /// structured meaning across the Lua boundary (currently
    /// [`Error::CallbackDestructed`] and [`Error::UserDataDestructed`]); plain
    /// string callback errors continue to surface as [`Error::RuntimeError`] for
    /// backward compatibility.
    ///
    /// [`Function::call`]: crate::Function::call
    CallbackError {
        /// A Lua call-stack traceback (empty when luaur-rt does not capture one).
        traceback: String,
        /// The original error returned by the Rust callback.
        cause: Arc<Error>,
    },
    /// A `UserData` could not be immutably borrowed because it is already
    /// mutably borrowed.
    UserDataBorrowError,
    /// A `UserData` could not be mutably borrowed because it is already
    /// borrowed.
    UserDataBorrowMutError,
    /// A coroutine ([`crate::Thread`]) could not be resumed because it has
    /// finished, errored, or is currently running. Mirrors
    /// `mlua::Error::CoroutineUnresumable`.
    CoroutineUnresumable,
    /// A [`crate::RegistryKey`] was used with a [`crate::Lua`] that does not
    /// own it. Mirrors `mlua::Error::MismatchedRegistryKey`.
    MismatchedRegistryKey,
    /// A `create_function_mut` / `create_userdata` mutable callback was invoked
    /// re-entrantly while a previous invocation still held the `&mut`. The inner
    /// `RefCell` borrow failed, which we surface as this variant rather than
    /// allowing mutable aliasing. Mirrors `mlua::Error::RecursiveMutCallback`.
    RecursiveMutCallback,
    /// A Rust panic was raised across a `pcall` boundary, caught and resumed
    /// once; a later attempt to re-raise/observe it failed because the panic was
    /// already resumed. Mirrors `mlua::Error::PreviouslyResumedPanic`.
    PreviouslyResumedPanic,
    /// An error originating outside Lua, wrapped via [`Error::external`].
    ExternalError(Arc<DynStdError>),
    /// A serialization (Rust -> Lua) error produced by the `serde` feature.
    /// Mirrors `mlua::Error::SerializeError`.
    #[cfg(feature = "serde")]
    SerializeError(String),
    /// A deserialization (Lua -> Rust) error produced by the `serde` feature.
    /// Mirrors `mlua::Error::DeserializeError`.
    #[cfg(feature = "serde")]
    DeserializeError(String),
    /// One or more static type-checker diagnostics produced by the `typecheck`
    /// feature (e.g. [`Lua::check`](crate::Lua::check) /
    /// [`Chunk::check`](crate::Chunk::check)). Each
    /// [`TypeDiagnostic`](crate::TypeDiagnostic) carries its 1-based source
    /// location.
    ///
    /// There is no mlua equivalent: Lua has no static types, so mlua cannot
    /// type-check a script before running it.
    #[cfg(feature = "typecheck")]
    TypeError(Vec<crate::TypeDiagnostic>),
}

#[cfg(feature = "serde")]
impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::SerializeError(msg.to_string())
    }
}

#[cfg(feature = "serde")]
impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::DeserializeError(msg.to_string())
    }
}

impl Error {
    /// Create a [`Error::RuntimeError`] from any displayable message.
    ///
    /// Mirrors `mlua::Error::runtime`.
    pub fn runtime<S: fmt::Display>(message: S) -> Self {
        Error::RuntimeError(message.to_string())
    }

    /// Try to view the wrapped external error as a concrete type `T`.
    ///
    /// Mirrors the common `mlua::Error::downcast_ref` use: only
    /// [`Error::ExternalError`] carries a wrapped error to downcast.
    pub fn downcast_ref<T: StdError + 'static>(&self) -> Option<&T> {
        match self {
            Error::ExternalError(e) => e.downcast_ref::<T>(),
            _ => None,
        }
    }

    /// Wrap an arbitrary `std::error::Error` as an [`Error::ExternalError`].
    ///
    /// Mirrors `mlua::Error::external`: if the input is already a luaur
    /// [`Error`], it is preserved as-is rather than re-wrapped.
    pub fn external<T: Into<Box<DynStdError>>>(err: T) -> Self {
        let boxed: Box<DynStdError> = err.into();
        // Preserve an already-`Error` value instead of nesting it.
        match boxed.downcast::<Error>() {
            Ok(e) => *e,
            Err(other) => Error::ExternalError(other.into()),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::SyntaxError { message, .. } => write!(f, "syntax error: {message}"),
            Error::RuntimeError(msg) => write!(f, "runtime error: {msg}"),
            Error::MemoryError(msg) => write!(f, "memory error: {msg}"),
            Error::FromLuaConversionError { from, to, message } => {
                write!(f, "error converting Lua {from} to {to}")?;
                if let Some(m) = message {
                    write!(f, " ({m})")?;
                }
                Ok(())
            }
            Error::ToLuaConversionError { from, to, message } => {
                write!(f, "error converting {from} to Lua {to}")?;
                if let Some(m) = message {
                    write!(f, " ({m})")?;
                }
                Ok(())
            }
            Error::UserDataTypeMismatch => write!(f, "userdata type mismatch"),
            Error::UserDataDestructed => write!(f, "userdata used after being destructed"),
            Error::CallbackDestructed => write!(
                f,
                "a destructed callback or destructed userdata method was called"
            ),
            Error::CallbackError { cause, .. } => write!(f, "{cause}"),
            Error::UserDataBorrowError => write!(f, "userdata already mutably borrowed"),
            Error::UserDataBorrowMutError => write!(f, "userdata already borrowed"),
            Error::CoroutineUnresumable => write!(f, "cannot resume this coroutine"),
            Error::MismatchedRegistryKey => {
                write!(f, "registry key used with the wrong Lua instance")
            }
            Error::RecursiveMutCallback => {
                write!(f, "mutable callback called recursively")
            }
            Error::PreviouslyResumedPanic => {
                write!(f, "previously resumed panic returned again")
            }
            Error::ExternalError(err) => write!(f, "{err}"),
            #[cfg(feature = "serde")]
            Error::SerializeError(msg) => write!(f, "serialize error: {msg}"),
            #[cfg(feature = "serde")]
            Error::DeserializeError(msg) => write!(f, "deserialize error: {msg}"),
            #[cfg(feature = "typecheck")]
            Error::TypeError(diagnostics) => {
                write!(f, "type error(s):")?;
                for d in diagnostics {
                    write!(f, "\n  {d}")?;
                }
                Ok(())
            }
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::ExternalError(err) => Some(&**err),
            Error::CallbackError { cause, .. } => Some(&**cause),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::external(err)
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Error::RuntimeError(msg.to_string())
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::RuntimeError(msg)
    }
}

/// Convenience for turning an arbitrary error/displayable into an [`Error`].
///
/// Mirrors `mlua::ExternalError`. `&str`/`String` become a [`Error::RuntimeError`]
/// (matching mlua's runtime-error semantics for string errors); other
/// `std::error::Error` types become an [`Error::ExternalError`].
pub trait ExternalError {
    /// Convert `self` into an [`Error`].
    fn into_lua_err(self) -> Error;
}

impl<E: Into<Box<DynStdError>>> ExternalError for E {
    fn into_lua_err(self) -> Error {
        // `&str`/`String`/`io::Error`/... all implement `Into<Box<dyn Error>>`.
        // Plain string errors become runtime errors (matching mlua); a wrapped
        // `mlua::Error` is preserved by `Error::external`.
        Error::external(self)
    }
}

/// `Result` extension mirroring `mlua::ExternalResult`: lift any
/// `Result<T, E>` into a `luaur` [`Result`] by converting the error.
pub trait ExternalResult<T> {
    /// Convert the error side via [`ExternalError::into_lua_err`].
    fn into_lua_err(self) -> Result<T>;
}

impl<T, E: ExternalError> ExternalResult<T> for std::result::Result<T, E> {
    fn into_lua_err(self) -> Result<T> {
        self.map_err(ExternalError::into_lua_err)
    }
}
