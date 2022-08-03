# rover
Rover is a script written in Rust which will find and unzip the most recent source code dowloaded from https://configure.zsa.io. It uses that source to generate code for qmk_firmware, compiles qmk_firmware, invokes Wally to flash the keyboard, then git-commits the changes locally. That is, after making changes to my keyboard configuration on configure.zsa.io, I simply run Rover to fully integrate those changes.

The primary purpose of this script is to automate the translation of macros entered into configure.zsa.io, which have a maximum of 5 characters, to their best matching full-length word.

Everything is hard-coded so you'll need to tweak this considerably before it'll work for you.
