#version 450

layout(location=0) in vec2 position;

layout(location=1) in vec3 model_matrix_0;
layout(location=2) in vec3 model_matrix_1;
layout(location=3) in vec3 model_matrix_2;

layout(location=4) in float radius;

layout(location=0) out vec2 v_position;
layout(location=1) out vec2 v_center;
layout(location=2) out float v_radius;

void main() {
  mat3 model_matrix = mat3(model_matrix_0, model_matrix_1, model_matrix_2);
  vec2 coords = (model_matrix * vec3(position, 0.0)).xy;
  vec2 center = vec2(model_matrix[2][0], model_matrix[2][1]);
  v_position = coords;
  v_center = center;
  v_radius = radius;
  gl_Position = vec4(coords, 0.0, 1.0);
}
