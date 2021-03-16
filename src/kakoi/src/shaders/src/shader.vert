#version 450

layout(location=0) in vec3 position;

layout(location=1) in vec4 model_matrix_0;
layout(location=2) in vec4 model_matrix_1;
layout(location=3) in vec4 model_matrix_2;
layout(location=4) in vec4 model_matrix_3;

// layout(location=5) in float radius;

layout(set=0, binding=0) uniform Uniforms { mat4 u_view_proj; };

// layout(location=0) out vec2 v_position;
// layout(location=1) out vec2 v_center;
// layout(location=2) out float v_radius;

void main() {
  mat4 model_matrix = mat4(model_matrix_0, model_matrix_1, model_matrix_2, model_matrix_3);
  mat4 transformation = u_view_proj * model_matrix;
  // vec4 p = transformation * vec4(position, 1.0);

  // vec2 coords = (model_matrix * vec3(position, 0.0)).xy;
  // vec2 center = vec2(model_matrix[2][0], model_matrix[2][1]);
  // v_position = p.xy;
  // v_position = position;
  // v_center = vec2(0.2, 0.3);
  // v_center = (transformation * vec4(0.0, 0.0, 0.0, 1.0)).xy;
  // v_center = vec2(0.0, 0.0);
  // float what = (transformation * vec4(1.0, 0.0, 0.0, 1.0)).x - v_center.x;
  // v_radius = what;
  // v_radius = what;
  gl_Position = transformation * vec4(position, 1.0);
}
