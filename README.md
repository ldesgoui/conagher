# conagher

`conagher` is a Rust library, it helps in the creation of server-side modifications for the game Team Fortress 2.
It is only compatible with Linux.

# How it works

Basically, `conagher` preloads itself into `dlopen`, which is used by `srcds_linux` to get the game's code libraries.
When those libraries are open, `conagher` analyses the contents for symbols.
Once those symbols are mapped to the library's memory, `conagher` applies patches (mostly detours) to modify the game's behaviour.


# Roadmap

- [ ] hooking CServerGameDLL::DLLInit
- [ ] symbols (needs more functionality)
- [ ] vtables
- [ ] entprop/datamap etc
- [ ] QOL for detours
- [ ] test: type safety for detours based on symbol
- [ ] Console {printing, commands, variables} support
- [ ] Plugins (loading during runtime)
- [ ] Web UI


# Mods

- [ ] tickrate modifier
- [ ] fix sticky det delay
- [ ] non-random pistol spread
- [ ] fix wallbugs
- [ ] fix splash bugs
- [ ] non-random fall damage
- [ ] projectiles must miss teammates
- [ ] better bots
- [ ] reimpl MGEMod/SOAP/tf2 training
- [ ] reimpl important TFTrue/CompCTRL/logs/demos features
- [ ] easier workshop support
- [ ] become the #1 tool for competitive TF2
