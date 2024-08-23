#version 140

in vec3 position;
in vec3 normal;
in vec3 color;

out vec3 vertex_color;
out vec3 vertex_normal;
out vec3 frag_pos;

uniform mat4 view_proj;
uniform mat4 model;
uniform mat3 normal_matrix;

void main() {
    vertex_color = color;
    vertex_normal = normal_matrix * normal;

    frag_pos = vec3(model * vec4(position, 1.0));
    gl_Position = view_proj * model * vec4(position, 1.0);
}