#version 140

in vec3 position;
in vec3 color;

out vec3 vertex_color;

uniform float offset;

void main() {
    vec3 pos = position;
    pos.x += offset;

    vertex_color = color;
    gl_Position = vec4(pos, 1.0);
}