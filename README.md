# vsop
Command line translation tool using [CTranslate2](https://github.com/OpenNMT/CTranslate2).

This tool employs a client-server approach to avoid loading the model every time a text needs to be translated.

## Basic Usage
Since this tool consists of a client and a server, the server `vsops` needs to be started first.
This command runs the server in the background:

```bash
vsops &
```

Then, run the client `vsop` to translate text. This command launches `vsop`,
which opens an editor for entering the text to be translated:

```bash
vsop
```

After saving and closing the editor, the text is translated.

## Compilation
This tool works on Linux and Mac, using several backend libraries such as
[OpenBLAS](https://www.openblas.net/),
[Intel MKL](https://www.intel.com/content/www/us/en/developer/tools/oneapi/onemkl.html),
[Ruy](https://github.com/google/ruy), and [Apple Accelerate](https://developer.apple.com/documentation/accelerate).

See [the `ct2rs` documentation](https://github.com/jkawamoto/ctranslate2-rs?tab=readme-ov-file#compilation)
for instructions on how to compile this tool using these backends.

## Commands
This tool provides the following commands.

### vsop
A client application for `vsops`, a translation server using CTranslate2.

This application opens an editor specified by the `EDITOR` environment variable and sends a translation request with
the written text to the server. If a file path is provided using the `--file` flag, the text in the file will be used
for the translation request. If the `--stdin` flag is given, it reads from stdin to obtain the text for translation.

To communicate with the server, it uses a domain socket. It attempts to connect to the socket file located in the
user's data directory. However, if a different path is specified with the `--socket-file` flag, it will connect to the
socket file at that specified location.

### vsops
A translation server using CTranslate2.

This server creates a UNIX domain socket, listens for translation requests, and handles them accordingly.

By default, a socket file is created in the user’s data directory. If the `--socket-file` flag is used to specify an
alternative path, the socket file will be created at that location.

The model specified by the `--model` flag will be downloaded from Hugging Face and loaded. If the `--model-dir` flag is
used to specify a directory path, the model within that directory will be loaded instead.

## License

This application is released under the MIT License. For details, see the [LICENSE](LICENSE) file.