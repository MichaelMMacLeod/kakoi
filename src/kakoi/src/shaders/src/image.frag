#version 450

layout(location=0) in vec2 v_texture_position;

layout(location=0) out vec4 color;

layout(set=0, binding=0) uniform texture2D t_diffuse;
layout(set=0, binding=1) uniform sampler s_diffuse;

void main() {
    color = texture(sampler2D(t_diffuse, s_diffuse), v_texture_position);
}