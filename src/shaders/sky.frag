#version 460

in float height;

out vec4 color;

uniform float max_height;
uniform vec3 low_color;
uniform vec3 high_color;

void main() {
    color = vec4(mix(low_color, high_color, height / max_height), 1.0);
}