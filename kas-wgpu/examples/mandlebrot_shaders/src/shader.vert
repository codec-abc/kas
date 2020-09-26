#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec2 a1;

layout(location = 0) out vec2 b1;

layout(set = 0, binding = 0) uniform Locals {
    vec2 scale;
};

const vec2 offset = { -1.0, 1.0 };

void main() {
    gl_Position = vec4(scale * a_pos.xy + offset, a_pos.z, 1.0);
    b1 = a1;
}