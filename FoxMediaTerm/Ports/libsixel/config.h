#pragma once

#define HAVE_ASSERT_H 1
#define HAVE_CLEARERR 1
#define HAVE_CLOCK 1
#define HAVE_CTYPE_H 1
#define HAVE_DEBUG 0
#define HAVE_ERRNO_H 1
#define HAVE_FCNTL_H 1
#define HAVE_FSEEK 1
#define HAVE_INTTYPES_H 1
#define HAVE_LDIV 1
#define HAVE_LIMITS_H 1
#define HAVE_LONGJMP 1
#define HAVE_MALLOC 1
#define HAVE_MATH_H 1
#define HAVE_MEMCPY 1
#define HAVE_MEMMOVE 1
#define HAVE_MEMORY_H 1
#define HAVE_REALLOC 1
#define HAVE_SETJMP 1
#define HAVE_SETJMP_H 1
#define HAVE_STAT 1
#define HAVE_STDDEF_H 1
#define HAVE_STDINT_H 1
#define HAVE_STDRING_H 0
#define HAVE_STRING_H 1
#define HAVE_SYS_STAT_H 1
#define HAVE_SYS_TYPES_H 1
#define HAVE_TESTS 0
#define HAVE_TIME_H 1
#define HAVE_UNISTD_H 1

#if defined(_WIN32)
#define HAVE_IO_H 1
#define HAVE_ISATTY 0
#define HAVE_SYS_TIME_H 0
#define HAVE_SYS_UNISTD_H 0
#define HAVE_SYS_IOCTL_H 0
#define HAVE_SYS_SELECT_H 0
#define HAVE_TERMIOS_H 0
#define HAVE__SETMODE 1
#define HAVE___BUILTIN_UNREACHABLE 0

#if defined(_MSC_VER) && !defined(__clang__)
#define __builtin_unreachable() __assume(0)
#endif
#else
#define HAVE_IO_H 0
#define HAVE_ISATTY 1
#define HAVE_NANOSLEEP 1
#define HAVE_SYS_IOCTL_H 1
#define HAVE_SYS_SELECT_H 1
#define HAVE_SYS_TIME_H 1
#define HAVE_SYS_UNISTD_H 0
#define HAVE_TERMIOS_H 1
#define HAVE__SETMODE 0
#define HAVE___BUILTIN_UNREACHABLE 1
#endif
