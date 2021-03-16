#version 450

// https://gamedev.stackexchange.com/questions/141264/an-efficient-way-for-generating-smooth-circle

layout(location=0) in vec2 position;
layout(location=1) in vec2 center;
layout(location=2) in float r;
layout(location=0) out vec4 color;

void main() {
  float thickness = 2.5;

  float radius = length(position - center);
  float signedDistance = radius - r;
  vec2 gradient = vec2(dFdx(signedDistance), dFdy(signedDistance));
  float rangeFromLine = abs(signedDistance / length(gradient));
  float lineWeight = clamp(thickness - rangeFromLine, 0.0, 1.0);

  // fancy colors: (doesn't display well on a white background)
  color = vec4(0.25 + 0.25 * position, 1.0, lineWeight * r * 1.5);
  // color = vec4(center, 1.0, 0.5);

  // boring colors:
  // color = vec4(0.0, 0.0, 0.0, lineWeight * r * 1.5);

  // other boring colors:
  // color = vec4(0.3, 0.3, 0.3, lineWeight);
}
