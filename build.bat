@echo off
aulua build
cargo build --release
copy target\release\vectorcanvas.dll build\vectorcanvas.mod2

mkdir C:\ProgramData\aviutl2\Script\VectorCanvas\
aulua install
copy build\vectorcanvas.mod2 C:\ProgramData\aviutl2\Script\VectorCanvas\
