#version 430 core

in layout(location=1) vec3 vertexNormal;
in layout(location=2) vec4 vertexColour;

vec3 lightDirection = normalize(vec3(0.8, -0.5, 0.6));

out vec4 color;

void main()
{
    color = vec4(vertexColour.rgb * max(0, dot(vertexNormal, -lightDirection)), 1.0f);
}