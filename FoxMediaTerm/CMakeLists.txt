cmake_minimum_required(VERSION 3.24)

project(FoxMediaTerm VERSION 0.1.0 LANGUAGES CXX)

include(GNUInstallDirs)
include(CMakePackageConfigHelpers)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

if(MSVC OR CMAKE_CXX_SIMULATE_ID STREQUAL "MSVC")
    set(FOX_MEDIA_TERM_OPTIMIZATION_FLAG "/O2")
    add_compile_options("$<$<OR:$<CONFIG:Release>,$<CONFIG:RelWithDebInfo>>:/O2>")
elseif(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang|AppleClang")
    set(FOX_MEDIA_TERM_OPTIMIZATION_FLAG "-O3")
    add_compile_options("$<$<OR:$<CONFIG:Release>,$<CONFIG:RelWithDebInfo>>:-O3>")
else()
    set(FOX_MEDIA_TERM_OPTIMIZATION_FLAG "compiler default")
endif()

# FFmpeg's vcpkg find module caches the aggregate library list. Clear it so
# manifest feature changes cannot leave swresample out of an existing build.
unset(FFMPEG_FOUND CACHE)
unset(FFMPEG_LIBRARIES CACHE)
unset(FFMPEG_LIBRARY CACHE)
find_package(FFMPEG REQUIRED)
find_package(Stb REQUIRED)
find_package(Threads REQUIRED)
find_package(unofficial-libsixel CONFIG REQUIRED)

find_path(MINIAUDIO_INCLUDE_DIR NAMES miniaudio.h REQUIRED)

add_library(
    FoxMediaTermImage STATIC
    Source/Image/AsciiGrayscaleRenderer.cpp
    Source/Image/Base64Encoder.cpp
    Source/Image/HalfBlock256Renderer.cpp
    Source/Image/HalfBlockTrueColorRenderer.cpp
    Source/Image/ITerm2Renderer.cpp
    Source/Image/ImageLoader.cpp
    Source/Image/ImagePrepareMetrics.cpp
    Source/Image/ImageRegionRenderer.cpp
    Source/Image/ImageResizer.cpp
    Source/Image/ImageViewer.cpp
    Source/Image/KittyRenderer.cpp
    Source/Image/PngEncoder.cpp
    Source/Image/SixelRenderer.cpp
    Source/Image/TerminalOutput.cpp
    Source/Image/TerminalProbe.cpp
    Source/Image/TerminalProtocolDetector.cpp
    Source/Image/TerminalGeometry.cpp
    Source/Image/TerminalSize.cpp
)
target_link_libraries(FoxMediaTermImage PRIVATE unofficial::libsixel::libsixel)
target_include_directories(
    FoxMediaTermImage
    PUBLIC "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/Source/Image>"
    PRIVATE "${Stb_INCLUDE_DIR}"
)

add_library(FoxMediaTermMedia STATIC Source/Media/MediaProbe.cpp)
target_include_directories(
    FoxMediaTermMedia
    PUBLIC "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/Source/Media>"
    PRIVATE ${FFMPEG_INCLUDE_DIRS}
)
target_link_directories(FoxMediaTermMedia PRIVATE ${FFMPEG_LIBRARY_DIRS})
target_link_libraries(FoxMediaTermMedia PRIVATE ${FFMPEG_LIBRARIES})

add_library(
    FoxMediaTermAudio STATIC
    Source/Audio/AudioDecoder.cpp
    Source/Audio/AudioDecoderInput.cpp
    Source/Audio/AudioDecoderResampling.cpp
    Source/Audio/AudioPcmRingBuffer.cpp
    Source/Audio/AudioPlayback.cpp
    Source/Audio/AudioPlaybackDevice.cpp
    Source/Audio/AudioPlaybackProducer.cpp
    Source/Audio/AudioVisualization.cpp
    Source/Audio/MiniaudioImplementation.cpp
)
target_include_directories(
    FoxMediaTermAudio
    PUBLIC "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/Source/Audio>"
    PRIVATE ${FFMPEG_INCLUDE_DIRS} "${MINIAUDIO_INCLUDE_DIR}"
)
target_link_directories(FoxMediaTermAudio PRIVATE ${FFMPEG_LIBRARY_DIRS})
target_link_libraries(
    FoxMediaTermAudio
    PRIVATE FoxMediaTermMedia ${FFMPEG_LIBRARIES} Threads::Threads ${CMAKE_DL_LIBS}
)

add_library(
    FoxMediaTermVideo STATIC
    Source/Video/PreparedVideoFrameBuffer.cpp
    Source/Video/VideoDecoder.cpp
    Source/Video/VideoDecoderInput.cpp
    Source/Video/VideoPlaybackClock.cpp
    Source/Video/VideoPlaybackReport.cpp
    Source/Video/VideoPlaybackTelemetry.cpp
    Source/Video/VideoPngEncoder.cpp
    Source/Video/VideoPresentation.cpp
    Source/Video/VideoPreviewPlayback.cpp
    Source/Video/VideoThumbnail.cpp
)
target_include_directories(
    FoxMediaTermVideo
    PUBLIC "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/Source/Video>"
    PRIVATE ${FFMPEG_INCLUDE_DIRS} "${MINIAUDIO_INCLUDE_DIR}"
)
target_link_directories(FoxMediaTermVideo PRIVATE ${FFMPEG_LIBRARY_DIRS})
target_link_libraries(
    FoxMediaTermVideo
    PUBLIC FoxMediaTermImage
    PRIVATE FoxMediaTermAudio FoxMediaTermMedia ${FFMPEG_LIBRARIES} Threads::Threads ${CMAKE_DL_LIBS}
)
if(UNIX)
    find_library(FOX_MEDIA_TERM_MATH_LIBRARY m)
    if(FOX_MEDIA_TERM_MATH_LIBRARY)
        target_link_libraries(FoxMediaTermAudio PRIVATE "${FOX_MEDIA_TERM_MATH_LIBRARY}")
        target_link_libraries(FoxMediaTermVideo PRIVATE "${FOX_MEDIA_TERM_MATH_LIBRARY}")
    endif()
endif()
target_compile_definitions(
    FoxMediaTermVideo
    PRIVATE
        FOX_MEDIA_TERM_BUILD_CONFIGURATION="$<CONFIG>"
        FOX_MEDIA_TERM_OPTIMIZATION_FLAG="${FOX_MEDIA_TERM_OPTIMIZATION_FLAG}"
)

include(FetchContent)

FetchContent_Declare(
    fpng
    GIT_REPOSITORY https://github.com/richgel999/fpng.git
    GIT_TAG 925796543b9d26b8edfcdcecd94c1dac280f29fc
    GIT_SHALLOW FALSE
    SOURCE_SUBDIR DoNotAddToBuild
)
FetchContent_Declare(
    fpnge
    GIT_REPOSITORY https://github.com/veluca93/fpnge.git
    GIT_TAG 097ebccf3d3db443d9a8f238b1deed010a82f961
    GIT_SHALLOW FALSE
    SOURCE_SUBDIR DoNotAddToBuild
)
FetchContent_MakeAvailable(fpng fpnge)

add_library(FoxMediaTermFpng STATIC "${fpng_SOURCE_DIR}/src/fpng.cpp")
target_include_directories(FoxMediaTermFpng PUBLIC "${fpng_SOURCE_DIR}/src")
target_compile_definitions(FoxMediaTermFpng PRIVATE FPNG_NO_SSE=1)

if(CMAKE_SYSTEM_PROCESSOR MATCHES "^(AMD64|amd64|x86_64|i[3-6]86)$")
    add_library(FoxMediaTermFpnge STATIC "${fpnge_SOURCE_DIR}/fpnge.cc")
    target_include_directories(FoxMediaTermFpnge PUBLIC "${fpnge_SOURCE_DIR}")
    target_compile_definitions(FoxMediaTermFpnge PUBLIC FOX_MEDIA_TERM_HAS_FPNGE=1)
    if(MSVC OR CMAKE_CXX_SIMULATE_ID STREQUAL "MSVC")
        target_compile_options(FoxMediaTermFpnge PRIVATE /arch:AVX2)
    elseif(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang|AppleClang")
        target_compile_options(FoxMediaTermFpnge PRIVATE -mavx2)
    endif()
else()
    add_library(FoxMediaTermFpnge INTERFACE)
    target_include_directories(FoxMediaTermFpnge INTERFACE "${fpnge_SOURCE_DIR}")
    target_compile_definitions(FoxMediaTermFpnge INTERFACE FOX_MEDIA_TERM_HAS_FPNGE=0)
endif()

target_link_libraries(FoxMediaTermVideo PRIVATE FoxMediaTermFpng FoxMediaTermFpnge)

add_library(
    FoxMediaTerm SHARED
    Source/FoxMediaTerm.cpp
    Source/Bridge/AudioBridge.cpp
    Source/Bridge/ImageBridge.cpp
    Source/Bridge/MediaPreview.cpp
    Source/Bridge/VideoBridge.cpp
)
add_library(FoxMediaTerm::FoxMediaTerm ALIAS FoxMediaTerm)
target_compile_definitions(FoxMediaTerm PRIVATE FOX_MEDIA_TERM_BUILD=1)
target_include_directories(
    FoxMediaTerm
    PUBLIC
        "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/Include>"
        "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
)
target_link_libraries(FoxMediaTerm PRIVATE FoxMediaTermAudio FoxMediaTermMedia FoxMediaTermVideo)
if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    # Static FFmpeg uses PC-relative references to global tables. Bind them locally
    # when embedding it in this shared library, as recommended by its vcpkg port.
    target_link_options(FoxMediaTerm PRIVATE "LINKER:-Bsymbolic")
endif()

set_target_properties(
    FoxMediaTerm
    PROPERTIES
        OUTPUT_NAME fox_media_term
        VERSION ${PROJECT_VERSION}
        SOVERSION ${PROJECT_VERSION_MAJOR}
        CXX_VISIBILITY_PRESET hidden
        VISIBILITY_INLINES_HIDDEN YES
        INSTALL_RPATH "$<$<PLATFORM_ID:Darwin>:@loader_path>$<$<NOT:$<PLATFORM_ID:Darwin>>:\$ORIGIN>"
)

include(CMake/CopyRuntimeDependencies.cmake)
FoxMediaTermInstallWithRuntimeDependencies(FoxMediaTerm)

install(
    FILES Include/FoxMediaTerm/FoxMediaTerm.h
    DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}/FoxMediaTerm"
)
install(
    EXPORT FoxMediaTermTargets
    FILE FoxMediaTermTargets.cmake
    NAMESPACE FoxMediaTerm::
    DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/FoxMediaTerm"
)
configure_package_config_file(
    CMake/FoxMediaTermConfig.cmake.in
    "${CMAKE_CURRENT_BINARY_DIR}/FoxMediaTermConfig.cmake"
    INSTALL_DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/FoxMediaTerm"
)
write_basic_package_version_file(
    "${CMAKE_CURRENT_BINARY_DIR}/FoxMediaTermConfigVersion.cmake"
    VERSION ${PROJECT_VERSION}
    COMPATIBILITY SameMajorVersion
)
install(
    FILES
        "${CMAKE_CURRENT_BINARY_DIR}/FoxMediaTermConfig.cmake"
        "${CMAKE_CURRENT_BINARY_DIR}/FoxMediaTermConfigVersion.cmake"
    DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/FoxMediaTerm"
)

include(CTest)
if(BUILD_TESTING)
    find_package(GTest CONFIG REQUIRED)

    add_executable(
        FoxMediaTermTests
        Tests/AudioTests.cpp
        Tests/ImagePipelineTests.cpp
        Tests/MediaBridgeTests.cpp
        Tests/TerminalProtocolDetectorTests.cpp
        Tests/VideoTests.cpp
    )
    target_link_libraries(
        FoxMediaTermTests
        PRIVATE
            FoxMediaTerm
            FoxMediaTermImage
            FoxMediaTermAudio
            FoxMediaTermMedia
            FoxMediaTermVideo
            GTest::gtest_main
            unofficial::libsixel::libsixel
    )

    include(GoogleTest)
    gtest_discover_tests(FoxMediaTermTests)
endif()

if(CMAKE_GENERATOR MATCHES "Ninja|Makefiles")
    add_custom_command(
        OUTPUT "${CMAKE_BINARY_DIR}/CompileCommandsCopy.stamp"
        COMMAND
            "${CMAKE_COMMAND}" -E copy_if_different "${CMAKE_BINARY_DIR}/compile_commands.json"
            "${CMAKE_SOURCE_DIR}/compile_commands.json"
        COMMAND "${CMAKE_COMMAND}" -E touch "${CMAKE_BINARY_DIR}/CompileCommandsCopy.stamp"
        DEPENDS "${CMAKE_BINARY_DIR}/compile_commands.json"
        COMMENT "Synchronizing compile_commands.json with the Fox Media Term root"
        VERBATIM
    )
    add_custom_target(CopyCompileCommands ALL DEPENDS "${CMAKE_BINARY_DIR}/CompileCommandsCopy.stamp")
endif()
