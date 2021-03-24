#version 450

layout(location=0) in vec3 position;

// wgpu-rs does not support vertex buffers containing matrices. We send vectors instead, and
// reassemble the matrix here.
layout(location=1) in vec4 model_matrix_0;
layout(location=2) in vec4 model_matrix_1;
layout(location=3) in vec4 model_matrix_2;
layout(location=4) in vec4 model_matrix_3;

layout(set=0, binding=0) uniform Uniforms { mat4 view_projection_matrix; };

void main() {
  mat4 model_matrix = mat4(model_matrix_0, model_matrix_1, model_matrix_2, model_matrix_3);
  mat4 transformation = view_projection_matrix * model_matrix;
  gl_Position = transformation * vec4(position, 1.0);
}