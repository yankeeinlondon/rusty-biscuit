use std::{collections::HashMap, sync::LazyLock};

use serde::{Deserialize, Serialize};

use crate::ProgramMetadata;

/// **Program**
///
/// An inventory of programs which this library is aware of and
/// has metadata for.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Program {
  // Editors
  Vi,
  Vim,
  Neovim,
  Emacs,
  Xemacs,
  Nano,
  Helix,
  VsCode,
  VsCodium,
  Sublime,
  Zed,
  Micro,
  Kakoune,
  Amp,
  Lapce,
  Phpstorm,
  IntellijIdea,
  Pycharm,
  Webstorm,
  Clion,
  Goland,
  Rider,
  Textmate,
  BbEdit,
  Geany,
  Kate,
}



pub static PROGRAM_LOOKUP: LazyLock<HashMap<Program, ProgramMetadata>> = LazyLock::new(|| {
    let lookup = HashMap::new();

    lookup.insert(Program::, ProgramMetadata {

    });


    lookup

});
