#version 450

layout(location=0) in vec3 position;
layout(location=1) in vec2 center;
layout(location=0) out vec3 v_position;
layout(location=1) out vec2 v_center;

void main() {
  v_position = position;
  v_center = center;
  gl_Position = vec4(position, 1.0);
}
