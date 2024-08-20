#version 140

in vec3 position;
in vec3 color;

out vec3 vertex_color;

uniform mat4 view_proj;

void main() {
    vertex_color = color;
    gl_Position = view_proj * vec4(position, 1.0);
}