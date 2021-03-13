#version 450

layout(location=0) in vec3 position;
layout(location=0) out vec3 v_position;

void main() {
  v_position = position;
  gl_Position = vec4(position, 1.0);
}
