cmake_minimum_required(VERSION 3.14)
project(cpp_ros2_node)

set(CMAKE_CXX_STANDARD 17)

include_directories(../../include)

add_executable(control_node src/control_node.cpp)
