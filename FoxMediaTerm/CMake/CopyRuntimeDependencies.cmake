function(FoxMediaTermInstallWithRuntimeDependencies target)
    install(
        TARGETS ${target}
        EXPORT FoxMediaTermTargets
        RUNTIME DESTINATION "${CMAKE_INSTALL_BINDIR}"
        LIBRARY DESTINATION "${CMAKE_INSTALL_LIBDIR}"
        ARCHIVE DESTINATION "${CMAKE_INSTALL_LIBDIR}"
    )

    install(CODE "
        set(FOX_MEDIA_TERM_RUNTIME_LIBRARY \"$<TARGET_FILE:${target}>\")
        set(FOX_MEDIA_TERM_RUNTIME_DESTINATION \"\${CMAKE_INSTALL_PREFIX}/${CMAKE_INSTALL_BINDIR}\")
        include(\"${CMAKE_CURRENT_FUNCTION_LIST_FILE}\")
    ")
endfunction()

if(DEFINED FOX_MEDIA_TERM_RUNTIME_LIBRARY AND WIN32)
    get_filename_component(runtime_directory "${FOX_MEDIA_TERM_RUNTIME_LIBRARY}" DIRECTORY)
    file(
        GET_RUNTIME_DEPENDENCIES
        LIBRARIES "${FOX_MEDIA_TERM_RUNTIME_LIBRARY}"
        DIRECTORIES "${runtime_directory}"
        RESOLVED_DEPENDENCIES_VAR resolved_dependencies
        UNRESOLVED_DEPENDENCIES_VAR unresolved_dependencies
        PRE_EXCLUDE_REGEXES "api-ms-win-.*" "ext-ms-win-.*"
        POST_EXCLUDE_REGEXES ".*[\\/]Windows[\\/]System32[\\/].*"
    )

    file(MAKE_DIRECTORY "${FOX_MEDIA_TERM_RUNTIME_DESTINATION}")
    foreach(dependency IN LISTS resolved_dependencies)
        cmake_path(IS_PREFIX runtime_directory "${dependency}" NORMALIZE is_app_local_dependency)
        if(is_app_local_dependency)
            get_filename_component(dependency_name "${dependency}" NAME)
            file(
                COPY_FILE "${dependency}"
                "${FOX_MEDIA_TERM_RUNTIME_DESTINATION}/${dependency_name}"
                ONLY_IF_DIFFERENT
            )
        endif()
    endforeach()
endif()
