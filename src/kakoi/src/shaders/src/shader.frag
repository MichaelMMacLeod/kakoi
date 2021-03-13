#version 450

// https://gamedev.stackexchange.com/questions/141264/an-efficient-way-for-generating-smooth-circle

layout(location=0) in vec3 position;
layout(location=1) in vec2 center;
layout(location=0) out vec4 color;

void main() {
  float thickness = 5.0;

  float radius = length(position.xy - center);
  float signedDistance = radius - 1.0;
  vec2 gradient = vec2(dFdx(signedDistance), dFdy(signedDistance));
  float rangeFromLine = abs(signedDistance / length(gradient));
  float lineWeight = clamp(thickness - rangeFromLine, 0.0, 1.0);

  // fancy colors:
  color = vec4(0.5 + 0.25 * position.xy, 1.0, lineWeight);

  // boring colors:
  // color = vec4(0.3, 0.3, 0.3, lineWeight);
}
