# AVIF decoder plugin

From this directory, build the plugin and copy it into the viewer's plugin directory:

```powershell
cargo build --release
Copy-Item target\release\avif_decoder_plugin.dll ..\avif_decoder.dll
```

The viewer loads `plugins\*.dll` at startup. Restart it after building or replacing the plugin.
The `image` 0.24 `avif-decoder` feature requires the native `dav1d` decoder and its build
dependencies on the build machine.

The plugin mirrors the host's Rust trait-object interface. Build it with the same Rust toolchain
and dependency versions as the viewer. This is an in-tree plugin interface, not a stable third-party
binary ABI.
