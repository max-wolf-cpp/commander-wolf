function(FoxMediaTermInstallWithRuntimeDependencies target)
    install(
        TARGETS ${target}
        EXPORT FoxMediaTermTargets
        RUNTIME DESTINATION "${CMAKE_INSTALL_BINDIR}"
        LIBRARY DESTINATION "${CMAKE_INSTALL_LIBDIR}"
        ARCHIVE DESTINATION "${CMAKE_INSTALL_LIBDIR}"
    )

    if(WIN32)
        set(runtime_destination "${CMAKE_INSTALL_BINDIR}")
        set(CMAKE_INSTALL_SYSTEM_RUNTIME_LIBS_SKIP TRUE)
        include(InstallRequiredSystemLibraries)
        if(CMAKE_INSTALL_SYSTEM_RUNTIME_LIBS)
            install(
                PROGRAMS ${CMAKE_INSTALL_SYSTEM_RUNTIME_LIBS}
                DESTINATION "${runtime_destination}"
                CONFIGURATIONS Release RelWithDebInfo MinSizeRel
            )
        endif()
    else()
        set(runtime_destination "${CMAKE_INSTALL_LIBDIR}")
    endif()

    install(CODE "
        set(FOX_MEDIA_TERM_RUNTIME_LIBRARY \"$<TARGET_FILE:${target}>\")
        set(FOX_MEDIA_TERM_RUNTIME_DESTINATION \"\${CMAKE_INSTALL_PREFIX}/${runtime_destination}\")
        include(\"${CMAKE_CURRENT_FUNCTION_LIST_FILE}\")
    ")
endfunction()

if(DEFINED FOX_MEDIA_TERM_RUNTIME_LIBRARY)
    get_filename_component(runtime_directory "${FOX_MEDIA_TERM_RUNTIME_LIBRARY}" DIRECTORY)
    if(WIN32)
        set(pre_excludes "api-ms-win-.*" "ext-ms-win-.*")
        set(post_excludes [[.*[\\/][Ss][Yy][Ss][Tt][Ee][Mm]32[\\/].*]])
    elseif(APPLE)
        set(pre_excludes "")
        set(post_excludes "^/usr/lib/" "^/System/Library/")
    else()
        set(pre_excludes "")
        set(post_excludes "^/lib/" "^/lib64/" "^/usr/lib/" "^/usr/lib64/")
    endif()

    file(
        GET_RUNTIME_DEPENDENCIES
        LIBRARIES "${FOX_MEDIA_TERM_RUNTIME_LIBRARY}"
        DIRECTORIES "${runtime_directory}"
        RESOLVED_DEPENDENCIES_VAR resolved_dependencies
        UNRESOLVED_DEPENDENCIES_VAR unresolved_dependencies
        PRE_EXCLUDE_REGEXES ${pre_excludes}
        POST_EXCLUDE_REGEXES ${post_excludes}
    )
    if(unresolved_dependencies)
        message(FATAL_ERROR "Cannot install unresolved Fox Media Term runtime dependencies: ${unresolved_dependencies}")
    endif()

    foreach(dependency IN LISTS resolved_dependencies)
        file(
            INSTALL "${dependency}"
            DESTINATION "${FOX_MEDIA_TERM_RUNTIME_DESTINATION}"
            TYPE SHARED_LIBRARY
            FOLLOW_SYMLINK_CHAIN
        )
    endforeach()

    if(APPLE)
        # Installing new rpaths invalidates the linker's ad-hoc signature on arm64.
        find_program(codesign_program codesign REQUIRED)
        file(GLOB installed_libraries "${FOX_MEDIA_TERM_RUNTIME_DESTINATION}/*.dylib")
        foreach(library IN LISTS installed_libraries)
            if(IS_SYMLINK "${library}")
                continue()
            endif()

            execute_process(
                COMMAND "${codesign_program}" --force --sign - "${library}"
                RESULT_VARIABLE sign_result
                ERROR_VARIABLE sign_error
            )
            if(NOT sign_result EQUAL 0)
                message(FATAL_ERROR "Cannot ad-hoc sign ${library}: ${sign_error}")
            endif()
        endforeach()
    endif()
endif()
