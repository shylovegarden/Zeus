import sys

with open('src/parser.rs', 'r') as f:
    content = f.read()

# 1. Add depth field to Parser
content = content.replace("    parsing_tensor_dims: bool,\\n}", "    parsing_tensor_dims: bool,\\n    depth: usize,\\n}")
content = content.replace("            parsing_tensor_dims: false,\\n        }", "            parsing_tensor_dims: false,\\n            depth: 0,\\n        }")

# 2. Rename parse_expression to parse_expression_internal
content = content.replace("    fn parse_expression(&mut self) -> Option<Expression> {\\n        let mut left",
                          """    fn parse_expression(&mut self) -> Option<Expression> {
        self.depth += 1;
        if self.depth > 128 {
            self.errors.push("Maximum recursion depth exceeded".to_string());
            self.depth -= 1;
            return None;
        }
        let res = self.parse_expression_internal();
        self.depth -= 1;
        res
    }

    fn parse_expression_internal(&mut self) -> Option<Expression> {
        let mut left""")

with open('src/parser.rs', 'w') as f:
    f.write(content)

print('Success Parser')
