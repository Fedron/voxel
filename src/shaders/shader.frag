#version 140

in vec3 vertex_color;
in vec3 vertex_normal;
in vec3 frag_pos;

out vec4 color;

uniform vec3 light_position;
uniform vec3 light_color;

void main() {
    float ambient_strength = 0.1;
    vec3 ambient = ambient_strength * light_color;

    vec3 norm = normalize(vertex_normal);
    vec3 light_dir = normalize(light_position - frag_pos);
    float diff = max(dot(norm, light_dir), 0.0);
    vec3 diffuse = diff * light_color;

    vec3 result = (ambient + diffuse) * vertex_color;
    color = vec4(pow(result, vec3(1.0 / 2.2)), 1.0);
}