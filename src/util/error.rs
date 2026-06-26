/**
 * Custom error structs to handle HTTP responses.
 */

use askama::Error as AskamaError;
use std::error::Error as StdError;
use std::any::Any as StdAny;
use std::marker::Send;

#[allow(unused)]
#[derive(Debug)]
pub enum RenderingError {
    Askama(AskamaError),
    Std(Box<dyn StdError>),
    Any(Box<dyn StdAny + Send>)
}

impl From<AskamaError> for RenderingError {
    fn from(err: AskamaError) -> Self {
        RenderingError::Askama(err)
    }
}

impl From<Box<dyn StdError>> for RenderingError {
    fn from(err: Box<dyn StdError>) -> Self {
        RenderingError::Std(err)
    }
}

impl From<Box<dyn StdAny + Send>> for RenderingError {
    fn from(err: Box<dyn StdAny + Send>) -> Self {
        RenderingError::Any(err)
    }
}