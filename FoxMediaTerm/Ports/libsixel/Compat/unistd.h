#pragma once

#include <fcntl.h>
#include <io.h>

#define close _close
#define fileno _fileno
#define isatty _isatty
#define open _open
#define read _read
#define write _write

#ifndef S_IRUSR
#define S_IRUSR _S_IREAD
#endif

#ifndef S_IWUSR
#define S_IWUSR _S_IWRITE
#endif

#ifndef STDIN_FILENO
#define STDIN_FILENO 0
#endif

#ifndef STDOUT_FILENO
#define STDOUT_FILENO 1
#endif

#ifndef STDERR_FILENO
#define STDERR_FILENO 2
#endif
