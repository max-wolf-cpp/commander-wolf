function(FoxMediaTermInstallLicenses)
    set(license_destination "${CMAKE_INSTALL_DATADIR}/fox_media_term/licenses")
    file(GLOB port_copyrights "${VCPKG_INSTALLED_DIR}/${VCPKG_TARGET_TRIPLET}/share/*/copyright")
    if(NOT port_copyrights)
        message(FATAL_ERROR "No vcpkg dependency copyright files found for ${VCPKG_TARGET_TRIPLET}")
    endif()

    foreach(copyright_file IN LISTS port_copyrights)
        get_filename_component(port_directory "${copyright_file}" DIRECTORY)
        get_filename_component(port_name "${port_directory}" NAME)
        install(FILES "${copyright_file}" DESTINATION "${license_destination}/vcpkg/${port_name}")
    endforeach()

    # fpng keeps its license in its public header rather than a separate LICENSE file.
    install(
        FILES "${fpng_SOURCE_DIR}/src/fpng.h" "${fpng_SOURCE_DIR}/src/fpng.cpp" "${fpng_SOURCE_DIR}/README.md"
        DESTINATION "${license_destination}/fpng"
    )
    install(FILES "${fpnge_SOURCE_DIR}/LICENSE" DESTINATION "${license_destination}/fpnge")
    install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/LICENSE" DESTINATION "${license_destination}")
    install(
        FILES "${CMAKE_CURRENT_SOURCE_DIR}/vcpkg.json" "${CMAKE_CURRENT_SOURCE_DIR}/vcpkg-configuration.json"
        DESTINATION "${license_destination}/manifests"
    )
endfunction()
