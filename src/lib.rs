/*
 * Copyright (c) 2022, Dragon's Zone Project. All rights reserved.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NON-INFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use std::convert::Infallible;
use std::fmt::{Debug, Display, Formatter};
use std::io::ErrorKind::Other;

pub enum SoftError {
    StdErr(std::fmt::Error),
    IoErr(std::io::Error),
    AppError(String),
}

impl Display for SoftError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SoftError::StdErr(err) => write!(f, "StdError: {}", err),
            SoftError::IoErr(err) => write!(f, "IOError: {}", err),
            SoftError::AppError(err) => write!(f, "{}", err),
        }
    }
}

impl Debug for SoftError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SoftError::StdErr(err) => write!(f, "{}", err),
            SoftError::IoErr(err) => write!(f, "{}", err),
            SoftError::AppError(err) => write!(f, "{}", err),
        }
    }
}

impl From<std::io::Error> for SoftError {
    fn from(value: std::io::Error) -> Self {
        Self::IoErr(value)
    }
}

impl From<Infallible> for SoftError {
    fn from(value: Infallible) -> Self {
        Self::IoErr(std::io::Error::new(Other, value))
    }
}

impl From<std::fmt::Error> for SoftError {
    fn from(value: std::fmt::Error) -> Self {
        Self::StdErr(value)
    }
}

impl std::error::Error for SoftError {}
