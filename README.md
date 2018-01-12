[![This picture contains original artwork for the game Team Fortress 2. The copyright for it is held by Valve Corporation, who created the software.](blob/master/header.png?raw=true)](http://www.teamfortress.com/engineerupdate/radigan/)

`conagher` is a Rust library, it helps in the creation of server-side modifications for the game Team Fortress 2.
It is only compatible with Linux.

# How it works

Basically, `conagher` preloads itself into `dlopen`, which is used by `srcds_linux` to get the game's code libraries.
When those libraries are open, `conagher` analyses the contents for symbols.
Once those symbols are mapped to the library's memory, `conagher` applies patches (mostly detours) to modify the game's behaviour.

# A note on licensing

`conagher` is, for now, copyrighted.
`header.png` contains original artwork for the game Team Fortress 2. The copyright for it is held by Valve Corporation, who created the software.

# Thanks

Thanks to @sigsegv-mvm for his libtf2mod.
