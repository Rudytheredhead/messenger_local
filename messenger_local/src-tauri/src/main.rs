// W buildzie release bez konsoli (inaczej Windows otwiera okno PowerShell/cmd przy starcie).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use messenger_local::run;

fn main() {
    run();
}
