cmake_minimum_required(VERSION 3.10)

set(PROJECT_NAME "cmdhub_rust")
project(${PROJECT_NAME} LANGUAGES CXX)

include("../cargokit/cmake/cargokit.cmake")
apply_cargokit(${PROJECT_NAME} ../../rust cmdhub_core "")

set(cmdhub_rust_bundled_libraries
  "${${PROJECT_NAME}_cargokit_lib}"
  PARENT_SCOPE
)
