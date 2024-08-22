#version 140

in vec3 position;
in vec3 normal;
in vec3 color;

out vec3 vertex_color;
out vec3 vertex_normal;
out vec3 frag_pos;

uniform mat4 view_proj;

void main() {
    vertex_color = color;
    vertex_normal = normal;

    // TODO: Use model matrix
    // TODO: Calculate frag position in world space
    frag_pos = position;
    gl_Position = view_proj * vec4(position, 1.0);
}