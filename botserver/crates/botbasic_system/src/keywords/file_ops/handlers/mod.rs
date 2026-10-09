/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.         |
| Licensed under the MIT License.                                             |
|                                                                             |
| This program is free software: you can redistribute it and/or modify        |
| it under the terms of the MIT License.                                      |
|                                                                             |
| The text of the MIT License can be found in the LICENSE file you have       |
| received along with this program.                                           |
|                                                                             |
| This program is distributed in the hope that it will be useful,             |
| but WITHOUT ANY WARRANTY, without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.                        |
|                                                                             |
| "General Bots" is a registered trademark of pragmatismo.com.br.             |
| The licensing of the program under the MIT License does not imply a         |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use log::trace;
pub(crate) use rhai::{Dynamic, Engine};
pub(crate) use std::sync::Arc;
pub(crate) use crate::keywords::file_ops::archive::*;
pub(crate) use crate::keywords::file_ops::basic_io::*;
pub(crate) use crate::keywords::file_ops::copy_move::*;
pub(crate) use crate::keywords::file_ops::create::*;
pub(crate) use crate::keywords::file_ops::create_shortcut::*;
pub(crate) use crate::keywords::file_ops::pdf::*;
pub(crate) use crate::keywords::file_ops::transfer::*;
pub(crate) use crate::keywords::file_ops::utils::dynamic_to_file_data;

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
pub use part3::*;
