(comment) @comment

(command_name
  (word) @function)

(function_definition
  name: (word) @function)

(variable_assignment
  name: (variable_name) @property)

(variable_assignment
  value: (_) @string)

[
  (string)
  (raw_string)
  (ansi_c_string)
] @string

(word) @string

((word) @boolean
  (#match? @boolean "^(true|false)$"))

((word) @constant.builtin
  (#match? @constant.builtin "^(null|none|all|latest|default)$"))

((word) @number
  (#match? @number "^[0-9]+(\\.[0-9]+)?$"))

((word) @variable.special
  (#match? @variable.special "^\\^[A-Za-z0-9_]+$"))

((word) @link_uri
  (#match? @link_uri "^https?://"))

[
  (number)
  (file_descriptor)
] @number

[
  "="
  "^"
  "+"
  "-"
  "/"
  ":"
  ";"
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket
