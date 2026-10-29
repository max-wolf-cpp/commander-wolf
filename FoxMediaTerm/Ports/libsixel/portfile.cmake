set(VCPKG_LIBRARY_LINKAGE static)

vcpkg_from_github(
    OUT_SOURCE_PATH SOURCE_PATH
    REPO saitoha/libsixel
    REF "v${VERSION}"
    SHA512 4f9d779732385af9cc81e7d9d823e6999a3cf653e6ce5e7eb19e7a7319736f960a31c9cd502e63c85f78907ab5375b0a310bb2deecd95b6f35769cff998a31b5
    HEAD_REF master
)

vcpkg_replace_string(
    "${SOURCE_PATH}/include/sixel.h.in"
    "#ifdef _WIN32\n# define SIXELAPI __declspec(dllexport)\n#else\n# define SIXELAPI\n#endif"
    "#if defined(_WIN32) && defined(SIXEL_SHARED)\n# if defined(SIXEL_BUILD)\n#  define SIXELAPI __declspec(dllexport)\n# else\n#  define SIXELAPI __declspec(dllimport)\n# endif\n#else\n# define SIXELAPI\n#endif\n\n#if defined(_MSC_VER)\n# define SIXEL_DEPRECATED __declspec(deprecated)\n#elif defined(__GNUC__) || defined(__clang__)\n# define SIXEL_DEPRECATED __attribute__((deprecated))\n#else\n# define SIXEL_DEPRECATED\n#endif"
)
vcpkg_replace_string(
    "${SOURCE_PATH}/include/sixel.h.in"
    "__attribute__((deprecated))"
    "SIXEL_DEPRECATED"
)

file(COPY "${CMAKE_CURRENT_LIST_DIR}/CMakeLists.txt" DESTINATION "${SOURCE_PATH}")
file(COPY "${CMAKE_CURRENT_LIST_DIR}/config.h" DESTINATION "${SOURCE_PATH}")
file(COPY "${CMAKE_CURRENT_LIST_DIR}/unofficial-libsixel-config.cmake.in" DESTINATION "${SOURCE_PATH}")
file(COPY "${CMAKE_CURRENT_LIST_DIR}/Compat" DESTINATION "${SOURCE_PATH}")

vcpkg_cmake_configure(SOURCE_PATH "${SOURCE_PATH}")
vcpkg_cmake_install()
vcpkg_cmake_config_fixup(PACKAGE_NAME unofficial-libsixel CONFIG_PATH lib/cmake/unofficial-libsixel)
vcpkg_copy_pdbs()

file(REMOVE_RECURSE "${CURRENT_PACKAGES_DIR}/debug/include")

vcpkg_install_copyright(FILE_LIST "${SOURCE_PATH}/LICENSE")
