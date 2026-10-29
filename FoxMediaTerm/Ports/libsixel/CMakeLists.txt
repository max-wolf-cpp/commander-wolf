cmake_minimum_required(VERSION 3.24)

project(libsixel VERSION 1.8.7 LANGUAGES C)

include(CMakePackageConfigHelpers)
include(GNUInstallDirs)

set(PACKAGE_VERSION "1.8.7-r2")
set(PACKAGE_DESCRIPTION "A lightweight, fast C implementation of DEC SIXEL graphics codec")
set(LS_LTVERSION "1:6:0")
configure_file(include/sixel.h.in Generated/sixel.h @ONLY)

add_library(
    sixel
    src/allocator.c
    src/chunk.c
    src/decoder.c
    src/dither.c
    src/encoder.c
    src/frame.c
    src/fromgif.c
    src/frompnm.c
    src/fromsixel.c
    src/loader.c
    src/malloc_stub.c
    src/output.c
    src/pixelformat.c
    src/quant.c
    src/scale.c
    src/status.c
    src/stb_image_write.c
    src/tosixel.c
    src/tty.c
    src/writer.c
)
set_target_properties(sixel PROPERTIES C_STANDARD 99 C_STANDARD_REQUIRED ON EXPORT_NAME libsixel)
target_include_directories(
    sixel
    PUBLIC
        "$<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}/Generated>"
        "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
    PRIVATE
        "${CMAKE_CURRENT_SOURCE_DIR}"
        "${CMAKE_CURRENT_SOURCE_DIR}/src"
)

if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    target_link_libraries(sixel PRIVATE m)
endif()

if(WIN32)
    target_include_directories(sixel PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/Compat")
    target_compile_definitions(sixel PRIVATE SIXEL_BUILD)

    if(BUILD_SHARED_LIBS)
        target_compile_definitions(sixel PUBLIC SIXEL_SHARED)
    endif()
endif()

install(
    TARGETS sixel
    EXPORT unofficial-libsixel-targets
    RUNTIME DESTINATION "${CMAKE_INSTALL_BINDIR}"
    LIBRARY DESTINATION "${CMAKE_INSTALL_LIBDIR}"
    ARCHIVE DESTINATION "${CMAKE_INSTALL_LIBDIR}"
)
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/Generated/sixel.h" DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}")
install(
    EXPORT unofficial-libsixel-targets
    FILE unofficial-libsixel-targets.cmake
    NAMESPACE unofficial::libsixel::
    DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/unofficial-libsixel"
)

configure_package_config_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/unofficial-libsixel-config.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/unofficial-libsixel-config.cmake"
    INSTALL_DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/unofficial-libsixel"
)
write_basic_package_version_file(
    "${CMAKE_CURRENT_BINARY_DIR}/unofficial-libsixel-config-version.cmake"
    VERSION "${PROJECT_VERSION}"
    COMPATIBILITY SameMajorVersion
)
install(
    FILES
        "${CMAKE_CURRENT_BINARY_DIR}/unofficial-libsixel-config.cmake"
        "${CMAKE_CURRENT_BINARY_DIR}/unofficial-libsixel-config-version.cmake"
    DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/unofficial-libsixel"
)
