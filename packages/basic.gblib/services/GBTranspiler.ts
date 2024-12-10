import { GBVMService } from './GBVMService';
import path from 'path';

interface Token {
  type: string;
  value: string;
  line: number;
  column: number;
}

interface ASTNode {
  type: string;
  value?: any;
  children?: ASTNode[];
  line: number;
  column: number;
}

export class GBCompiler {
  private source: string = '';
  private tokens: Token[] = [];
  private currentToken = 0;
  private line = 1;
  private column = 1;
  private output: string[] = [];
  private inString = false;
  private stringChar = '';
  private buffer = '';
  private lineMap: {[key: number]: number} = {};
  private tasks: any[] = [];

  // Internal state tracking
  private inTalkBlock = false;
  private talkBuffer = '';
  private inSystemPrompt = false; 
  private systemPromptBuffer = '';
  private currentTable: any = null;

  constructor() {
    // Initialize state
  }

  public compile(source: string, options: {
    filename: string;
    mainName: string;
    pid: string;
  }): {
    code: string;
    map: {[key: number]: number};
    metadata: any;
    tasks: any[];
    systemPrompt: string;
  } {
    this.source = source;
    this.reset();

    // Compilation phases
    this.tokenize();
    const ast = this.parse(); 
    this.generateCode(ast);

    return {
      code: this.output.join('\n'),
      map: this.lineMap,
      metadata: this.generateMetadata(options.mainName),
      tasks: this.tasks,
      systemPrompt: this.systemPromptBuffer
    };
  }

  private reset() {
    this.tokens = [];
    this.currentToken = 0;
    this.line = 1;
    this.column = 1;
    this.output = [];
    this.lineMap = {};
    this.tasks = [];
    this.inTalkBlock = false;
    this.talkBuffer = '';
    this.inSystemPrompt = false;
    this.systemPromptBuffer = '';
    this.currentTable = null;
  }

  private tokenize() {
    let current = 0;
    const source = this.source;

    while (current < source.length) {
      let char = source[current];

      // Handle whitespace
      if (/\s/.test(char)) {
        if (char === '\n') {
          this.line++;
          this.column = 1;
          
          if (this.buffer) {
            this.addToken('WORD', this.buffer);
            this.buffer = '';
          }
          
          this.addToken('NEWLINE', '\n');
        }
        current++;
        this.column++;
        continue;
      }

      // Handle strings
      if (char === '"' || char === "'" || char === '`') {
        if (this.buffer) {
          this.addToken('WORD', this.buffer);
          this.buffer = '';
        }

        const stringValue = this.readString(source, current, char);
        this.addToken('STRING', stringValue.value);
        current = stringValue.position;
        continue;
      }

      // Handle operators
      if ('=()[]{}+-*/%<>!&|,'.includes(char)) {
        if (this.buffer) {
          this.addToken('WORD', this.buffer);
          this.buffer = '';
        }

        const operator = this.readOperator(source, current);
        this.addToken('OPERATOR', operator.value);
        current = operator.position;
        continue;
      }

      // Build up word buffer
      this.buffer += char;
      current++;
      this.column++;
    }

    // Add any remaining buffer
    if (this.buffer) {
      this.addToken('WORD', this.buffer);
    }

    this.addToken('EOF', '');
  }

  private readString(source: string, start: number, quote: string) {
    let value = '';
    let position = start + 1;
    let escaped = false;

    while (position < source.length) {
      const char = source[position];
      
      if (escaped) {
        value += char;
        escaped = false;
      } else if (char === '\\') {
        escaped = true;
      } else if (char === quote) {
        position++;
        break;
      } else {
        value += char;
      }
      
      position++;
    }

    return { value, position };
  }

  private readOperator(source: string, start: number) {
    const twoCharOperators = ['==', '!=', '>=', '<=', '&&', '||'];
    const oneChar = source[start];
    const twoChar = oneChar + (source[start + 1] || '');

    if (twoCharOperators.includes(twoChar)) {
      return {
        value: twoChar,
        position: start + 2
      };
    }

    return {
      value: oneChar,
      position: start + 1
    };
  }

  private addToken(type: string, value: string) {
    this.tokens.push({
      type,
      value,
      line: this.line,
      column: this.column
    });
  }

  private parse(): ASTNode {
    const program: ASTNode = {
      type: 'Program',
      children: [],
      line: 1,
      column: 1
    };

    while (this.currentToken < this.tokens.length) {
      const statement = this.parseStatement();
      if (statement) {
        program.children.push(statement);
      }
    }

    return program;
  }

  private parseWhileStatement(): ASTNode {
    this.currentToken++; // Skip DO
    
    if (this.peek().value.toUpperCase() !== 'WHILE') {
      throw new Error('Expected WHILE after DO');
    }
    this.currentToken++; // Skip WHILE

    const condition = this.parseCondition();
    const body = this.parseBlock();

    // Must end with LOOP
    if (this.peek().value.toUpperCase() !== 'LOOP') {
      throw new Error('Expected LOOP at end of while statement');
    }
    this.currentToken++;

    return {
      type: 'WhileStatement',
      value: {
        condition,
        body
      },
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private parseExpression(): ASTNode {
    return this.parseAssignment();
  }

  private parseAssignment(): ASTNode {
    const left = this.parseLogicalOr();

    if (this.peek().value === '=') {
      this.currentToken++;
      const right = this.parseAssignment();
      return {
        type: 'AssignmentExpression',
        value: { left, right },
        line: this.tokens[this.currentToken].line,
        column: this.tokens[this.currentToken].column
      };
    }

    return left;
  }

  private parseLogicalOr(): ASTNode {
    let left = this.parseLogicalAnd();

    while (
      this.peek().value.toUpperCase() === 'OR' || 
      this.peek().value === '||'
    ) {
      const operator = this.tokens[this.currentToken].value;
      this.currentToken++;
      const right = this.parseLogicalAnd();
      left = {
        type: 'LogicalExpression',
        value: { operator: '||', left, right },
        line: this.tokens[this.currentToken].line,
        column: this.tokens[this.currentToken].column
      };
    }

    return left;
  }

  private parseLogicalAnd(): ASTNode {
    let left = this.parseEquality();

    while (
      this.peek().value.toUpperCase() === 'AND' || 
      this.peek().value === '&&'
    ) {
      const operator = this.tokens[this.currentToken].value;
      this.currentToken++;
      const right = this.parseEquality();
      left = {
        type: 'LogicalExpression',
        value: { operator: '&&', left, right },
        line: this.tokens[this.currentToken].line,
        column: this.tokens[this.currentToken].column
      };
    }

    return left;
  }

  private parseEquality(): ASTNode {
    let left = this.parseRelational();

    while (
      this.peek().value === '=' || 
      this.peek().value === '<>' ||
      this.peek().value === '==' ||
      this.peek().value === '!='
    ) {
      let operator = this.tokens[this.currentToken].value;
      this.currentToken++;
      
      // Convert BASIC operators to JS
      if (operator === '=') operator = '===';
      if (operator === '<>') operator = '!==';
      
      const right = this.parseRelational();
      left = {
        type: 'BinaryExpression',
        value: { operator, left, right },
        line: this.tokens[this.currentToken].line,
        column: this.tokens[this.currentToken].column
      };
    }

    return left;
  }

  private parseRelational(): ASTNode {
    let left = this.parseAdditive();

    while (
      this.peek().value === '<' ||
      this.peek().value === '>' ||
      this.peek().value === '<=' ||
      this.peek().value === '>='
    ) {
      const operator = this.tokens[this.currentToken].value;
      this.currentToken++;
      const right = this.parseAdditive();
      left = {
        type: 'BinaryExpression',
        value: { operator, left, right },
        line: this.tokens[this.currentToken].line,
        column: this.tokens[this.currentToken].column
      };
    }

    return left;
  }

  private parseAdditive(): ASTNode {
    let left = this.parseMultiplicative();

    while (
      this.peek().value === '+' ||
      this.peek().value === '-'
    ) {
      const operator = this.tokens[this.currentToken].value;
      this.currentToken++;
      const right = this.parseMultiplicative();
      left = {
        type: 'BinaryExpression',
        value: { operator, left, right },
        line: this.tokens[this.currentToken].line,
        column: this.tokens[this.currentToken].column
      };
    }

    return left;
  }

  private parseMultiplicative(): ASTNode {
    let left = this.parseUnary();

    while (
      this.peek().value === '*' ||
      this.peek().value === '/' ||
      this.peek().value === '%'
    ) {
      const operator = this.tokens[this.currentToken].value;
      this.currentToken++;
      const right = this.parseUnary();
      left = {
        type: 'BinaryExpression',
        value: { operator, left, right },
        line: this.tokens[this.currentToken].line,
        column: this.tokens[this.currentToken].column
      };
    }

    return left;
  }

  private parseUnary(): ASTNode {
    if (
      this.peek().value === '-' ||
      this.peek().value === '!' ||
      this.peek().value.toUpperCase() === 'NOT'
    ) {
      const operator = this.tokens[this.currentToken].value;
      this.currentToken++;
      const argument = this.parseUnary();
      return {
        type: 'UnaryExpression',
        value: { 
          operator: operator.toUpperCase() === 'NOT' ? '!' : operator, 
          argument 
        },
        line: this.tokens[this.currentToken].line,
        column: this.tokens[this.currentToken].column
      };
    }

    return this.parsePrimary();
  }

  private parsePrimary(): ASTNode {
    const token = this.peek();

    switch (token.type) {
      case 'NUMBER':
        this.currentToken++;
        return {
          type: 'NumberLiteral',
          value: parseFloat(token.value),
          line: token.line,
          column: token.column
        };

      case 'STRING':
        this.currentToken++;
        return {
          type: 'StringLiteral',
          value: token.value,
          line: token.line,
          column: token.column
        };

      case 'WORD':
        // Check for special keywords
        switch (token.value.toUpperCase()) {
          case 'TRUE':
          case 'FALSE':
            this.currentToken++;
            return {
              type: 'BooleanLiteral',
              value: token.value.toUpperCase() === 'TRUE',
              line: token.line,
              column: token.column
            };

          case 'NULL':
            this.currentToken++;
            return {
              type: 'NullLiteral',
              value: null,
              line: token.line,
              column: token.column
            };

          default:
            // Check if it's a function call
            if (this.tokens[this.currentToken + 1]?.value === '(') {
              return this.parseFunctionCall();
            }
            // Otherwise it's an identifier
            this.currentToken++;
            return {
              type: 'Identifier',
              value: token.value,
              line: token.line,
              column: token.column
            };
        }

      case 'LEFT_PAREN':
        this.currentToken++; // Skip (
        const expr = this.parseExpression();
        if (this.peek().value !== ')') {
          throw new Error('Expected closing parenthesis');
        }
        this.currentToken++; // Skip )
        return expr;

      default:
        throw new Error(
          `Unexpected token ${token.type} "${token.value}" at line ${token.line} column ${token.column}`
        );
    }
  }

  private parseFunctionCall(): ASTNode {
    const identifier = this.parseIdentifier();
    
    if (this.peek().value !== '(') {
      throw new Error('Expected ( after function name');
    }
    this.currentToken++; // Skip (

    const args = [];
    while (this.peek().value !== ')') {
      args.push(this.parseExpression());
      
      if (this.peek().value === ',') {
        this.currentToken++;
      } else if (this.peek().value !== ')') {
        throw new Error('Expected , or ) in function call');
      }
    }
    this.currentToken++; // Skip )

    return {
      type: 'FunctionCall',
      value: {
        callee: identifier,
        arguments: args
      },
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private parseForStatement(): ASTNode {
    this.currentToken++; // Skip FOR
    
    const variable = this.parseIdentifier();
    
    if (this.peek().value !== '=') {
      throw new Error('Expected = in FOR loop initialization');
    }
    this.currentToken++;
    
    const start = this.parseExpression();
    
    if (this.peek().value.toUpperCase() !== 'TO') {
      throw new Error('Expected TO in FOR loop');
    }
    this.currentToken++;
    
    const end = this.parseExpression();
    let step = null;

    // Optional STEP
    if (this.peek().value.toUpperCase() === 'STEP') {
      this.currentToken++;
      step = this.parseExpression();
    }

    const body = this.parseBlock();

    // Must end with NEXT
    if (this.peek().value.toUpperCase() !== 'NEXT') {
      throw new Error('Expected NEXT at end of FOR loop');
    }
    this.currentToken++;

    return {
      type: 'ForStatement',
      value: {
        variable,
        start,
        end,
        step,
        body
      },
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private parseForEachStatement(): ASTNode {
    this.currentToken++; // Skip FOR
    
    if (this.peek().value.toUpperCase() !== 'EACH') {
      throw new Error('Expected EACH after FOR');
    }
    this.currentToken++;

    const variable = this.parseIdentifier();
    
    if (this.peek().value.toUpperCase() !== 'IN') {
      throw new Error('Expected IN in FOR EACH statement');
    }
    this.currentToken++;

    const collection = this.parseExpression();
    const body = this.parseBlock();

    // Must end with NEXT
    if (this.peek().value.toUpperCase() !== 'NEXT') {
      throw new Error('Expected NEXT at end of FOR EACH loop');
    }
    this.currentToken++;

    return {
      type: 'ForEachStatement', 
      value: {
        variable,
        collection,
        body
      },
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private parseFunctionStatement(): ASTNode {
    this.currentToken++; // Skip FUNCTION
    
    const name = this.parseIdentifier();
    
    if (this.peek().value !== '(') {
      throw new Error('Expected ( after function name');
    }
    this.currentToken++;
    
    const params = this.parseParameterList();
    
    if (this.peek().value !== ')') {
      throw new Error('Expected ) after parameter list');
    }
    this.currentToken++;

    const body = this.parseBlock();

    // Must end with END FUNCTION
    if (this.peek().value.toUpperCase() !== 'END' || 
        this.tokens[this.currentToken + 1].value.toUpperCase() !== 'FUNCTION') {
      throw new Error('Expected END FUNCTION');
    }
    this.currentToken += 2;

    return {
      type: 'FunctionDeclaration',
      value: {
        name,
        params,
        body
      },
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private parseSelectStatement(): ASTNode {
    this.currentToken++; // Skip SELECT
    
    const fields = this.parseSelectList();
    
    if (this.peek().value.toUpperCase() !== 'FROM') {
      throw new Error('Expected FROM in SELECT statement');
    }
    this.currentToken++;

    const tableName = this.parseIdentifier();
    let whereClause = null;

    // Optional WHERE clause
    if (this.peek().value.toUpperCase() === 'WHERE') {
      this.currentToken++;
      whereClause = this.parseExpression();
    }

    return {
      type: 'SelectStatement',
      value: {
        fields,
        tableName,
        whereClause
      },
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private parseChartStatement(): ASTNode {
    this.currentToken++; // Skip CHART
    
    const type = this.parseExpression();
    
    if (this.peek().value !== ',') {
      throw new Error('Expected , after chart type');
    }
    this.currentToken++;

    const data = this.parseExpression();
    let legends = null;
    let transpose = null;
    let prompt = null;

    // Check for additional parameters
    while (this.peek().value === ',') {
      this.currentToken++;
      
      if (this.peek().value.toUpperCase() === 'LEGENDS') {
        this.currentToken++;
        legends = this.parseExpression();
      } else if (this.peek().value.toUpperCase() === 'TRANSPOSE') {
        this.currentToken++;
        transpose = this.parseExpression();
      } else if (this.peek().value.toUpperCase() === 'PROMPT') {
        this.currentToken++;
        prompt = this.parseExpression();
      }
    }

    return {
      type: 'ChartStatement',
      value: {
        type,
        data,
        legends,
        transpose,
        prompt
      },
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private parseHearStatement(): ASTNode {
    this.currentToken++; // Skip HEAR
    
    const variable = this.parseIdentifier();
    let kind = null;
    let args = null;

    // Check for AS clause
    if (this.peek().value.toUpperCase() === 'AS') {
      this.currentToken++;
      kind = this.parseIdentifier();
      
      // Optional arguments
      if (this.peek().value === ',') {
        this.currentToken++;
        args = this.parseExpressionList();
      }
    }

    return {
      type: 'HearStatement',
      value: {
        variable,
        kind,
        args
      },
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column  
    };
  }

  private parseTalkStatement(): ASTNode {
    this.currentToken++; // Skip TALK
    
    const text = this.parseExpression();

    return {
      type: 'TalkStatement',
      value: text,
      line: this.tokens[this.currentToken].line, 
      column: this.tokens[this.currentToken].column
    };
  }

  // Helper parsers
  private parseIdentifier(): {value: string, line: number, column: number} {
    const token = this.tokens[this.currentToken];
    if (token.type !== 'WORD') {
      throw new Error(`Expected identifier but got ${token.type}`);
    }
    this.currentToken++;
    return {
      value: token.value,
      line: token.line,
      column: token.column
    };
  }

  private parseExpressionList(): ASTNode[] {
    const expressions = [];
    
    do {
      expressions.push(this.parseExpression());
      
      if (this.peek().value !== ',') {
        break;
      }
      this.currentToken++;
    } while (true);

    return expressions;
  }

  private parseParameterList(): string[] {
    const params = [];
    
    while (this.peek().value !== ')') {
      params.push(this.parseIdentifier().value);
      
      if (this.peek().value !== ',') {
        break;
      }
      this.currentToken++;
    }

    return params;
  }

  private parseSelectList(): string[] {
    const fields = [];
    
    do {
      fields.push(this.parseIdentifier().value);
      
      if (this.peek().value !== ',') {
        break;
      }
      this.currentToken++; 
    } while (true);

    return fields;
  }
  private parseTalkBlock(): ASTNode {
    // Check if we're starting a talk block
    if (!this.inTalkBlock && this.peek().value.toUpperCase() === 'BEGIN' && 
        this.tokens[this.currentToken + 1]?.value.toUpperCase() === 'TALK') {
      this.currentToken += 2; // Skip BEGIN TALK
      this.inTalkBlock = true;
      this.talkBuffer = '';
      return null;
    }

    // Check if we're ending a talk block
    if (this.inTalkBlock && this.peek().value.toUpperCase() === 'END' && 
        this.tokens[this.currentToken + 1]?.value.toUpperCase() === 'TALK') {
      this.currentToken += 2; // Skip END TALK
      this.inTalkBlock = false;
      
      return {
        type: 'TalkBlock',
        value: this.talkBuffer.trim(),
        line: this.tokens[this.currentToken].line,
        column: this.tokens[this.currentToken].column
      };
    }

    // If we're inside a talk block, accumulate text
    if (this.inTalkBlock) {
      const line = this.tokens[this.currentToken].value;
      this.talkBuffer += line + '\n';
      this.currentToken++;
      return null;
    }

    throw new Error(
      `Unexpected token in TALK block at line ${this.tokens[this.currentToken].line}`
    );
  }

  private parseSystemPromptBlock(): ASTNode {
    // Check if we're starting a system prompt block
    if (!this.inSystemPrompt && this.peek().value.toUpperCase() === 'BEGIN' && 
        this.tokens[this.currentToken + 1]?.value.toUpperCase() === 'SYSTEM' &&
        this.tokens[this.currentToken + 2]?.value.toUpperCase() === 'PROMPT') {
      this.currentToken += 3; // Skip BEGIN SYSTEM PROMPT
      this.inSystemPrompt = true;
      this.systemPromptBuffer = '';
      return null;
    }

    // Check if we're ending a system prompt block
    if (this.inSystemPrompt && this.peek().value.toUpperCase() === 'END' && 
        this.tokens[this.currentToken + 1]?.value.toUpperCase() === 'SYSTEM' &&
        this.tokens[this.currentToken + 2]?.value.toUpperCase() === 'PROMPT') {
      this.currentToken += 3; // Skip END SYSTEM PROMPT
      this.inSystemPrompt = false;
      
      return {
        type: 'SystemPromptBlock',
        value: this.systemPromptBuffer.trim(),
        line: this.tokens[this.currentToken].line,
        column: this.tokens[this.currentToken].column
      };
    }

    // If we're inside a system prompt block, accumulate text
    if (this.inSystemPrompt) {
      const line = this.tokens[this.currentToken].value;
      this.systemPromptBuffer += line + '\n';
      this.currentToken++;
      return null;
    }

    throw new Error(
      `Unexpected token in SYSTEM PROMPT block at line ${this.tokens[this.currentToken].line}`
    );
  }

  private parseTableDefinition(): ASTNode {
    // Check if we're starting a table definition
    if (!this.currentTable && this.peek().value.toUpperCase() === 'TABLE') {
      this.currentToken++; // Skip TABLE
      
      const tableName = this.parseIdentifier();
      
      if (this.peek().value.toUpperCase() !== 'ON') {
        throw new Error('Expected ON after table name');
      }
      this.currentToken++; // Skip ON
      
      const connection = this.parseIdentifier();
      
      this.currentTable = {
        name: tableName.value,
        connection: connection.value,
        fields: {}
      };
      
      return null;
    }

    // Check if we're ending a table definition
    if (this.currentTable && this.peek().value.toUpperCase() === 'END' &&
        this.tokens[this.currentToken + 1]?.value.toUpperCase() === 'TABLE') {
      this.currentToken += 2; // Skip END TABLE
      
      const table = this.currentTable;
      this.currentTable = null;
      
      return {
        type: 'TableDefinition',
        value: table,
        line: this.tokens[this.currentToken].line,
        column: this.tokens[this.currentToken].column
      };
    }

    // If we're inside a table definition, parse field definition
    if (this.currentTable) {
      const field = this.parseFieldDefinition();
      if (field) {
        this.currentTable.fields[field.name] = field.definition;
      }
      this.currentToken++;
      return null;
    }

    throw new Error(
      `Unexpected token in TABLE definition at line ${this.tokens[this.currentToken].line}`
    );
  }

  private parseFieldDefinition(): { name: string; definition: any } | null {
    const token = this.peek();
    
    // Skip empty lines or comments
    if (token.type === 'NEWLINE' || 
        token.value.startsWith("'") || 
        token.value.toUpperCase().startsWith('REM')) {
      return null;
    }

    const name = this.parseIdentifier();
    
    if (this.peek().value.toUpperCase() !== 'AS') {
      throw new Error('Expected AS in field definition');
    }
    this.currentToken++; // Skip AS

    const type = this.parseIdentifier();
    let length = null;
    let precision = null;
    let scale = null;

    // Check for size specification
    if (this.peek().value === '(') {
      this.currentToken++; // Skip (
      length = this.parseNumber();
      
      // Check for decimal precision
      if (this.peek().value === ',') {
        this.currentToken++; // Skip ,
        precision = length;
        scale = this.parseNumber();
        length = null;
      }
      
      if (this.peek().value !== ')') {
        throw new Error('Expected ) in field size specification');
      }
      this.currentToken++; // Skip )
    }

    return {
      name: name.value,
      definition: {
        type: type.value,
        length,
        precision,
        scale
      }
    };
  }

  private parseNumber(): number {
    const token = this.peek();
    if (token.type !== 'NUMBER') {
      throw new Error('Expected number');
    }
    this.currentToken++;
    return parseFloat(token.value);
  }
  
  private parseStatement(): ASTNode {
    const token = this.tokens[this.currentToken];

    // Skip empty lines
    if (token.type === 'NEWLINE') {
      this.currentToken++;
      return null;
    }

    // Handle special blocks first
    if (this.inTalkBlock) {
      return this.parseTalkBlock();
    }

    if (this.inSystemPrompt) {
      return this.parseSystemPromptBlock();
    }

    if (this.currentTable) {
      return this.parseTableDefinition();
    }

    // Match tokens to specific statement types
    switch (token.value.toUpperCase()) {
      case 'INPUT':
        return this.parseInputStatement();
      
      case 'PRINT':
        return this.parsePrintStatement();
      
      case 'WRITE':
        return this.parseWriteStatement();
      
      case 'REM':
        return this.parseRemStatement();
      
      case 'CLOSE':
        return this.parseCloseStatement();
      
      case 'OPEN':
        return this.parseOpenStatement();
      
      case 'SELECT':
        return this.parseSelectStatement();
      
      case 'IF':
        return this.parseIfStatement();
      
      case 'FUNCTION':
        return this.parseFunctionStatement();
      
      case 'FOR':
        return this.parseForStatement();
      
      case 'DO':
        return this.parseDoWhileStatement();
      
      case 'WHILE':
        return this.parseWhileStatement();
      
      case 'TALK':
        return this.parseTalkStatement();
      
      case 'HEAR':
        return this.parseHearStatement();

      // Handle assignment statements
      default:
        if (this.isAssignment()) {
          return this.parseAssignmentStatement();
        }
        
        throw new Error(`Unexpected token ${token.value} at line ${token.line}`);
    }
  }

  private parseInputStatement(): ASTNode {
    this.currentToken++; // Skip INPUT
    
    const prompt = this.parseExpression();
    let variable = null;

    if (this.peek().value === ',') {
      this.currentToken++; // Skip comma
      variable = this.parseIdentifier();
    }

    return {
      type: 'InputStatement',
      value: {
        prompt,
        variable
      },
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private parsePrintStatement(): ASTNode {
    this.currentToken++; // Skip PRINT
    
    const expression = this.parseExpression();
    let sessionName = null;

    // Handle special #file printing
    if (expression.value && expression.value.toString().startsWith('#')) {
      sessionName = expression.value.substr(1, expression.value.indexOf(',') - 1);
      const items = expression.value.substr(expression.value.indexOf(',') + 1)
        .split(/[,;]/)
        .map(item => item.trim());

      return {
        type: 'FilePrintStatement',
        value: {
          sessionName,
          items
        },
        line: this.tokens[this.currentToken].line,
        column: this.tokens[this.currentToken].column
      };
    }

    return {
      type: 'PrintStatement',
      value: expression,
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private parseOpenStatement(): ASTNode {
    this.currentToken++; // Skip OPEN
    
    const filepath = this.parseExpression();
    let mode = null;
    let sessionName = null;

    // Handle FOR APPEND/OUTPUT
    if (this.peek().value.toUpperCase() === 'FOR') {
      this.currentToken++;
      mode = this.parseIdentifier().value;
    }

    // Handle AS #num or WITH #name
    if (this.peek().value.toUpperCase() === 'AS' || 
        this.peek().value.toUpperCase() === 'WITH') {
      const kind = this.peek().value.toUpperCase();
      this.currentToken += 2; // Skip AS/WITH and #
      sessionName = this.parseExpression();

      return {
        type: 'OpenFileStatement',
        value: {
          filepath,
          mode,
          kind,
          sessionName
        },
        line: this.tokens[this.currentToken].line,
        column: this.tokens[this.currentToken].column
      };
    }

    return {
      type: 'OpenStatement',
      value: {
        filepath,
        mode
      },
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private parseIfStatement(): ASTNode {
    this.currentToken++; // Skip IF
    
    const condition = this.parseCondition();
    
    // Must have THEN
    if (this.peek().value.toUpperCase() !== 'THEN') {
      throw new Error('Expected THEN after IF condition');
    }
    this.currentToken++;

    const thenBlock = this.parseBlock();
    let elseBlock = null;

    // Optional ELSE
    if (this.peek().value.toUpperCase() === 'ELSE') {
      this.currentToken++;
      elseBlock = this.parseBlock();
    }

    // Must end with END IF
    if (this.peek().value.toUpperCase() !== 'END' || 
        this.tokens[this.currentToken + 1].value.toUpperCase() !== 'IF') {
      throw new Error('Expected END IF');
    }
    this.currentToken += 2;

    return {
      type: 'IfStatement',
      value: {
        condition,
        thenBlock,
        elseBlock
      },
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private parseCondition(): ASTNode {
    const condition = this.parseExpression();
    
    // Convert BASIC comparisons to JS
    if (condition.type === 'BinaryExpression') {
      switch (condition.value.operator) {
        case '=':
          condition.value.operator = '===';
          break;
        case '<>':
          condition.value.operator = '!==';
          break;
      }
    }

    return condition;
  }

  private parseBlock(): ASTNode[] {
    const statements: ASTNode[] = [];
    
    while (this.currentToken < this.tokens.length) {
      const token = this.peek();
      
      if (['END', 'ELSE'].includes(token.value.toUpperCase())) {
        break;
      }

      const statement = this.parseStatement();
      if (statement) {
        statements.push(statement);
      }
    }

    return statements;
  }
  private generateIfCode(ast: ASTNode): void {
    const { condition, thenBlock, elseBlock } = ast.value;

    // Convert the condition and emit the if statement
    const convertedCondition = this.generateCondition(condition);
    this.emitCode(`if (${convertedCondition}) {`);
    
    // Indent for then block
    this.indent++;
    
    // Generate code for the then block
    thenBlock.forEach(statement => {
      this.generateCode(statement);
    });
    
    this.indent--;

    // Handle optional else block
    if (elseBlock) {
      this.emitCode('} else {');
      this.indent++;
      
      elseBlock.forEach(statement => {
        this.generateCode(statement);
      });
      
      this.indent--;
    }

    this.emitCode('}');
  }

  private generateCondition(condition: ASTNode): string {
    switch (condition.type) {
      case 'BinaryExpression':
        return this.generateBinaryCondition(condition);
      
      case 'LogicalExpression':
        return this.generateLogicalCondition(condition);
      
      case 'UnaryExpression':
        return this.generateUnaryCondition(condition);
      
      case 'ParenthesizedExpression':
        return `(${this.generateCondition(condition.value)})`;
      
      case 'Identifier':
        return condition.value;
      
      case 'NumberLiteral':
      case 'StringLiteral':
      case 'BooleanLiteral':
        return this.generateLiteral(condition);
      
      case 'FunctionCall':
        return this.generateFunctionCall(condition);
      
      default:
        throw new Error(`Unknown condition type: ${condition.type}`);
    }
  }

  private generateBinaryCondition(condition: ASTNode): string {
    const { operator, left, right } = condition.value;
    
    // Convert BASIC operators to JavaScript
    const jsOperator = this.convertOperator(operator);
    
    return `${this.generateCondition(left)} ${jsOperator} ${this.generateCondition(right)}`;
  }

  private generateLogicalCondition(condition: ASTNode): string {
    const { operator, left, right } = condition.value;
    
    // Convert BASIC logical operators to JavaScript
    const jsOperator = this.convertLogicalOperator(operator);
    
    return `${this.generateCondition(left)} ${jsOperator} ${this.generateCondition(right)}`;
  }

  private generateUnaryCondition(condition: ASTNode): string {
    const { operator, argument } = condition.value;
    
    // Convert BASIC unary operators to JavaScript
    const jsOperator = this.convertUnaryOperator(operator);
    
    return `${jsOperator}${this.generateCondition(argument)}`;
  }

  private convertOperator(operator: string): string {
    switch (operator.toUpperCase()) {
      case '=': return '===';
      case '<>': return '!==';
      case '>=': return '>=';
      case '<=': return '<=';
      case '>': return '>';
      case '<': return '<';
      default: return operator;
    }
  }

  private convertLogicalOperator(operator: string): string {
    switch (operator.toUpperCase()) {
      case 'AND': return '&&';
      case 'OR': return '||';
      case '&&': return '&&';
      case '||': return '||';
      default: return operator;
    }
  }

  private convertUnaryOperator(operator: string): string {
    switch (operator.toUpperCase()) {
      case 'NOT': return '!';
      case '!': return '!';
      case '-': return '-';
      default: return operator;
    }
  }

  private generateLiteral(literal: ASTNode): string {
    switch (literal.type) {
      case 'NumberLiteral':
        return literal.value.toString();
      
      case 'StringLiteral':
        return `"${literal.value}"`;
      
      case 'BooleanLiteral':
        return literal.value.toString();
      
      default:
        throw new Error(`Unknown literal type: ${literal.type}`);
    }
  }

  private generateFunctionCall(call: ASTNode): string {
    const { callee, arguments: args } = call.value;
    const generatedArgs = args.map(arg => this.generateCondition(arg)).join(', ');
    return `${callee.value}(${generatedArgs})`;
  }

  // Indentation handling
  private indent: number = 0;
  private getIndentation(): string {
    return '  '.repeat(this.indent);
  }


  // Handle special BASIC condition functions
  private isIntrinsicConditionFunction(name: string): boolean {
    return [
      'ISNULL', 
      'ISNUMERIC', 
      'ISDATE', 
      'ISEMPTY',
      'CONTAINS',
      'STARTSWITH',
      'ENDSWITH'
    ].includes(name.toUpperCase());
  }

  private generateIntrinsicCondition(call: ASTNode): string {
    const { callee, arguments: args } = call.value;
    
    switch (callee.value.toUpperCase()) {
      case 'ISNULL':
        return `${this.generateCondition(args[0])} === null`;
      
      case 'ISNUMERIC':
        return `!isNaN(parseFloat(${this.generateCondition(args[0])}))`;
      
      case 'ISDATE':
        return `!isNaN(Date.parse(${this.generateCondition(args[0])}))`;
      
      case 'ISEMPTY':
        return `${this.generateCondition(args[0])} === ""`;
      
      case 'CONTAINS':
        return `${this.generateCondition(args[0])}.includes(${this.generateCondition(args[1])})`;
      
      case 'STARTSWITH':
        return `${this.generateCondition(args[0])}.startsWith(${this.generateCondition(args[1])})`;
      
      case 'ENDSWITH':
        return `${this.generateCondition(args[0])}.endsWith(${this.generateCondition(args[1])})`;
      
      default:
        throw new Error(`Unknown intrinsic function: ${callee.value}`);
    }
  }
  private generateForCode(ast: ASTNode): void {
    const { variable, start, end, step, body } = ast.value;

    if (this.isForEach(ast)) {
      this.generateForEachCode(ast);
      return;
    }

    // Generate standard FOR loop initialization
    const initValue = this.generateExpression(start);
    const endValue = this.generateExpression(end);
    const stepValue = step ? this.generateExpression(step) : '1';
    const varName = this.generateExpression(variable);

    // Generate the for loop with proper direction check
    this.emitCode(`for (let ${varName} = ${initValue}; ` +
      `${stepValue} > 0 ? ${varName} <= ${endValue} : ${varName} >= ${endValue}; ` +
      `${varName} += ${stepValue}) {`);

    // Indent and generate loop body
    this.indent++;
    body.forEach(statement => {
      this.generateCode(statement);
    });
    this.indent--;

    this.emitCode('}');
  }

  private generateForEachCode(ast: ASTNode): void {
    const { variable, collection, body } = ast.value;

    // Special handling for collection with paging
    if (this.hasPagedCollection(collection)) {
      this.generatePagedForEachCode(ast);
      return;
    }

    // Generate initialization code
    this.emitCode(`
      __totalCalls = 10;
      __next = true;
      __calls = 0;
      __data = ${this.generateExpression(collection)};
      __index = 0;

      if (__data[0] && __data[0]['gbarray']) {
        __data = __data.slice(1);
      }

      __pageMode = __data?.pageMode ? __data.pageMode : "none";
      __url = __data?.links?.next?.uri;
      __seekToken = __data.links?.self?.headers["MS-ContinuationToken"];
      __totalCount = __data?.totalCount ? __data.totalCount : __data.length;

      while (__next && __totalCount) {
        let ${this.generateExpression(variable)} = __data?.items ? 
          __data?.items[__index] : __data[__index];
    `);

    // Indent and generate loop body
    this.indent++;
    body.forEach(statement => {
      this.generateCode(statement);
    });
    this.indent--;

    // Generate paging logic
    this.emitCode(`
        __index = __index + 1;

        if (__index === __totalCount) {
          if (__calls < __totalCalls && __pageMode === "auto") {
            let ___data = null;
            await retry(async (bail) => {
              await ensureTokens();
              ___data = await sys.getHttp({
                pid: pid,
                file: __url,
                addressOrHeaders: headers,
                httpUsername,
                httpPs
              });
            }, { retries: 5 });

            __data = ___data;
            ___data = null;

            __url = __data?.links?.next?.uri;
            __seekToken = __data?.links?.self?.headers["MS-ContinuationToken"];
            __totalCount = __data?.totalCount ? __data.totalCount : __data.length;

            __index = 0;
            __calls++;
          } else {
            __next = false;
          }
        }
        __data = null;
      }`);
  }

  private generatePagedForEachCode(ast: ASTNode): void {
    const { variable, collection, pageSize, body } = ast.value;

    this.emitCode(`
      if (!limit) limit = ${pageSize || 100};
      __page = 1;
      while (__page > 0 && __page < pages) {
        let __res = null;
        await retry(async (bail) => {
          await ensureTokens();
          __res = await sys.getHttp({
            pid: pid,
            file: host + ${this.generateExpression(collection.url)} + 
                  '?' + pageVariable + '=' + __page + 
                  '&' + limitVariable + '=' + limit,
            addressOrHeaders: headers,
            httpUsername,
            httpPs
          });
        }, { retries: 5 });

        await sleep(330);

        res = __res;
        __res = null;
        list1 = res.data;
        res = null;

        let j1 = 0;
        items1 = [];
        while (j1 < ubound(list1)) {
          let ${this.generateExpression(variable)} = list1[j1];
    `);

    // Indent and generate loop body
    this.indent++;
    body.forEach(statement => {
      this.generateCode(statement);
    });
    this.indent--;

    // Generate paging footer
    this.emitCode(`
          j1 = j1 + 1;
        }

        list1 = null;
        __page = list1?.length < limit ? 0 : __page + 1;
      }`);
  }

  private isForEach(ast: ASTNode): boolean {
    return ast.type === 'ForEachStatement';
  }

  private hasPagedCollection(collection: ASTNode): boolean {
    return collection.type === 'PagedCollection';
  }

  private generateExpression(expr: ASTNode): string {
    switch (expr.type) {
      case 'Identifier':
        return expr.value;

      case 'NumberLiteral':
        return expr.value.toString();

      case 'StringLiteral':
        return `"${expr.value}"`;

      case 'BinaryExpression':
        return this.generateBinaryExpression(expr);

      case 'FunctionCall':
        return this.generateFunctionCall(expr);

      default:
        throw new Error(`Unknown expression type: ${expr.type}`);
    }
  }

  private generateBinaryExpression(expr: ASTNode): string {
    const { operator, left, right } = expr.value;
    return `${this.generateExpression(left)} ${operator} ${this.generateExpression(right)}`;
  }

  // Special handling for collection operations
  private generateCollectionOperation(operation: ASTNode): string {
    const { method, args } = operation.value;
    
    switch (method) {
      case 'FILTER':
        return `filter(${args.map(arg => this.generateExpression(arg)).join(', ')})`;
      
      case 'MAP':
        return `map(${args.map(arg => this.generateExpression(arg)).join(', ')})`;
      
      case 'REDUCE':
        return `reduce(${args.map(arg => this.generateExpression(arg)).join(', ')})`;
      
      default:
        throw new Error(`Unknown collection operation: ${method}`);
    }
  }

  // Loop control statement handling
  private generateContinue(): void {
    this.emitCode('continue;');
  }

  private generateBreak(): void {
    this.emitCode('break;');
  }

  private generateExit(): void {
    this.emitCode('break;');
  }


  private generateCode(ast: ASTNode): void {
    switch (ast.type) {
      case 'Program':
        ast.children?.forEach(child => this.generateCode(child));
        break;

      case 'InputStatement':
        this.generateInputCode(ast);
        break;

      case 'PrintStatement':
        this.generatePrintCode(ast);
        break;

      case 'FilePrintStatement': 
        this.generateFilePrintCode(ast);
        break;

      case 'OpenStatement':
        this.generateOpenCode(ast);
        break;

      case 'IfStatement':
        this.generateIfCode(ast);
        break;

      case 'ForStatement':
        this.generateForCode(ast);
        break;

      case 'WhileStatement':
        this.generateWhileCode(ast);
        break;

      case 'ForEachStatement':
        this.generateForEachCode(ast);
        break;

      case 'HearStatement':
        this.generateHearCode(ast);
        break;

      case 'TalkStatement':
        this.generateTalkCode(ast);
        break;

      case 'AssignmentStatement':
        this.generateAssignmentCode(ast);
        break;
    }
  }

  private generateInputCode(ast: ASTNode): void {
    const {prompt, variable} = ast.value;
    
    if (variable) {
      this.emitCode(`
        TALK ${this.generateExpression(prompt)}
        HEAR ${this.generateExpression(variable)}
      `);
    } else {
      this.emitCode(`HEAR ${this.generateExpression(prompt)}`);
    }
  }

  private generatePrintCode(ast: ASTNode): void {
    this.emitCode(`await dk.talk({pid: pid, text: ${this.generateExpression(ast.value)}})`);
  }

  private generateFilePrintCode(ast: ASTNode): void {
    const {sessionName, items} = ast.value;

    if (items.length > 1) {
      this.emitCode(
        `await sys.save({pid: pid, file: files[${sessionName}], args:[${items.join(',')}]})`
      );
    } else {
      this.emitCode(
        `await sys.set({pid: pid, file: files[${sessionName}], address: col++, name: "${items[0]}", value: ${items[0]}})`
      );
    }
  }
  private parseDoWhileStatement(): ASTNode {
    this.currentToken++; // Skip DO
    
    if (this.peek().value.toUpperCase() !== 'WHILE') {
      throw new Error(`Expected WHILE after DO at line ${this.line}`);
    }
    this.currentToken++; // Skip WHILE

    const condition = this.parseCondition();
    const body = this.parseBlock();

    // Must end with LOOP
    if (this.peek().value.toUpperCase() !== 'LOOP') {
      throw new Error(`Expected LOOP at line ${this.line}`);
    }
    this.currentToken++; // Skip LOOP

    return {
      type: 'WhileStatement',
      value: {
        condition,
        body
      },
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private isAssignment(): boolean {
    // Look ahead to check for assignment pattern
    let i = this.currentToken;
    
    // Skip identifier(s)
    while (i < this.tokens.length && 
           (this.tokens[i].type === 'WORD' || 
            this.tokens[i].value === '.' || 
            this.tokens[i].value === '[' || 
            this.tokens[i].value === ']')) {
      i++;
    }

    // Check if next non-whitespace token is =
    while (i < this.tokens.length) {
      if (this.tokens[i].type !== 'WHITESPACE') {
        return this.tokens[i].value === '=';
      }
      i++;
    }

    return false;
  }

  private generateWhileCode(ast: ASTNode): void {
    const { condition, body } = ast.value;

    // Convert the condition
    const convertedCondition = this.generateCondition(condition);
    
    // Emit while loop
    this.emitCode(`while (${convertedCondition}) {`);
    
    // Indent and generate loop body
    this.indent++;
    body.forEach(statement => {
      this.generateCode(statement);
    });
    this.indent--;

    this.emitCode('}');
  }

  private parseAssignmentStatement(): ASTNode {
    let left = this.parseLeftHandSide();
    
    if (this.peek().value !== '=') {
      throw new Error(`Expected = but got ${this.peek().value} at line ${this.line}`);
    }
    this.currentToken++; // Skip =

    // Handle special assignments
    const nextToken = this.peek();
    let right;

    switch (nextToken.value.toUpperCase()) {
      case 'SELECT':
        right = this.parseSelectStatement();
        break;

      case 'CHART':
        right = this.parseChartStatement();
        break;

      case 'GET':
        right = this.parseGetStatement();
        break;

      case 'POST':
        right = this.parseHttpStatement('POST');
        break;

      case 'PUT':
        right = this.parseHttpStatement('PUT');
        break;

      case 'BLUR':
        right = this.parseImageOperation('BLUR');
        break;

      case 'SHARPEN':
        right = this.parseImageOperation('SHARPEN');
        break;

      case 'FORMAT':
        right = this.parseFormatStatement();
        break;

      case 'DATEDIFF':
        right = this.parseDateOperation('DATEDIFF');
        break;

      case 'DATEADD':
        right = this.parseDateOperation('DATEADD');
        break;

      case 'NEW':
        right = this.parseNewStatement();
        break;

      case 'FIND':
        right = this.parseFindStatement();
        break;

      case 'CREATE':
        if (this.tokens[this.currentToken + 1]?.value.toUpperCase() === 'DEAL') {
          right = this.parseCreateDealStatement();
        } else {
          right = this.parseCreateStatement();
        }
        break;

      case 'ACTIVE':
        if (this.tokens[this.currentToken + 1]?.value.toUpperCase() === 'TASKS') {
          right = this.parseActiveTasksStatement();
        } else {
          right = this.parseExpression();
        }
        break;

      case 'UPLOAD':
        right = this.parseUploadStatement();
        break;

      case 'FILL':
        right = this.parseFillStatement();
        break;

      case 'CARD':
        right = this.parseCardStatement();
        break;

      case 'ALLOW':
        if (this.tokens[this.currentToken + 1]?.value.toUpperCase() === 'ROLE') {
          right = this.parseAllowRoleStatement();
        } else {
          right = this.parseExpression();
        }
        break;

      default:
        right = this.parseExpression();
    }

    return {
      type: 'AssignmentStatement',
      value: {
        left,
        right,
        operator: '='
      },
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private parseLeftHandSide(): ASTNode {
    let identifier = this.parseIdentifier();
    
    // Handle property access (obj.prop) and array access (arr[index])
    while (
      this.peek().value === '.' || 
      this.peek().value === '[') {
      if (this.peek().value === '.') {
        this.currentToken++; // Skip .
        const property = this.parseIdentifier();
        identifier = {
          type: 'MemberExpression',
          value: {
            object: identifier,
            property,
            computed: false
          },
          line: this.tokens[this.currentToken].line,
          column: this.tokens[this.currentToken].column
        };
      } else {
        this.currentToken++; // Skip [
        const index = this.parseExpression();
        
        if (this.peek().value !== ']') {
          throw new Error(`Expected ] at line ${this.line}`);
        }
        this.currentToken++; // Skip ]
        
        identifier = {
          type: 'MemberExpression',
          value: {
            object: identifier,
            property: index,
            computed: true
          },
          line: this.tokens[this.currentToken].line,
          column: this.tokens[this.currentToken].column
        };
      }
    }

    return identifier;
  }

  // Helper method to handle async operations in assignments
  private isAsyncOperation(rightType: string): boolean {
    const asyncOperations = [
      'SelectStatement',
      'HttpRequest',
      'ImageOperation',
      'ChartStatement',
      'FindStatement',
      'UploadStatement',
      'CreateDealStatement',
      'ActiveTasksStatement',
      'AllowRoleStatement',
      'FillStatement',
      'CardStatement'
    ];
    return asyncOperations.includes(rightType);
  }

  private generateAssignmentCode(ast: ASTNode): void {
    const { left, right, operator } = ast.value;
    const leftCode = this.generateExpression(left);
    
    // Check if this is an async operation
    if (this.isAsyncOperation(right.type)) {
      this.emitCode(`${leftCode} = await ${this.generateAsyncOperation(right)};`);
    } else {
      this.emitCode(`${leftCode} = ${this.generateExpression(right)};`);
    }
  }

  private generateAsyncOperation(operation: ASTNode): string {
    switch (operation.type) {
      case 'SelectStatement':
        return this.generateSelectOperation(operation);
      case 'HttpRequest':
        return this.generateHttpOperation(operation);
      case 'ImageOperation':
        return this.generateImageOperation(operation);
      case 'ChartStatement':
        return this.generateChartOperation(operation);
      // ... handle other async operations
      default:
        throw new Error(`Unknown async operation type: ${operation.type}`);
    }
  }
  
  private generateOpenCode(ast: ASTNode): void {
    let {filepath, mode, kind, sessionName} = ast.value;

    if (kind === 'AS' && this.isNumber(sessionName)) {
      const filename = `${filepath.substr(0, filepath.lastIndexOf('.'))}.xlsx`;
      this.emitCode(`
        col = 1
        await sys.save({pid: pid, file: "${filename}", args: [id]})
        await dk.setFilter({pid: pid, value: "id=" + id})
        files[${sessionName}] = "${filename}"
      `);
    } else {
      const params = this.generateParams(['url', 'username', 'password'], [filepath]);
      sessionName = sessionName ? `"${sessionName}"` : null;
      const kindStr = `"${kind}"`;
      
      this.emitCode(
        `page = await wa.openPage({pid: pid, handle: page, sessionKind: ${kindStr}, sessionName: ${sessionName}, ${params}})`
      );
    }
  }



  private generateHearCode(ast: ASTNode): void {
    const {variable, kind, args} = ast.value;

    if (kind) {
      if (kind === 'sheet') {
        this.emitCode(
          `${variable} = await dk.hear({pid: pid, kind:"sheet", arg: "${args[0]}"})`
        );
      } else {
        this.emitCode(
          `${variable} = await dk.hear({pid: pid, kind:"${kind}"${args ? `, args: [${args}]` : ''}})`
        );
      }
    } else {
      this.emitCode(`${variable} = await dk.hear({pid: pid})`);
    }
  }

  private generateTalkCode(ast: ASTNode): void {
    const text = this.normalizeQuotes(ast.value);
    this.emitCode(`await dk.talk({pid: pid, text: ${text}})`);
  }
  private generateChartAssignment(left: string, right: ASTNode): void {
    const {type, data, legends, transpose, prompt} = right.value;
    
    // Handle regular chart
    if (!prompt) {
      this.emitCode(`
        ${left} = await dk.chart({
          pid: pid,
          type: ${type},
          data: ${data},
          legends: ${legends},
          transpose: ${transpose}
        })`
      );
    } 
    // Handle chart with prompt (LLM chart)
    else {
      this.emitCode(`
        ${left} = await dk.llmChart({
          pid: pid,
          type: ${type},
          data: ${data},
          prompt: ${prompt}
        })`
      );
    }
  }

  private generateSelectAssignment(left: string, right: ASTNode): void {
    const {tableName, sql} = right.value;
    
    // Replace table name with ? in SQL
    const sqlWithPlaceholder = sql.replace(tableName, '?');

    this.emitCode(`
      ${left} = await sys.executeSQL({
        pid: pid,
        data: ${tableName},
        sql: \`${sqlWithPlaceholder}\`
      })`
    );
  }

  private generateHttpAssignment(left: string, right: ASTNode): void {
    const {method, url, data} = right.value;

    switch (method.toUpperCase()) {
      case 'GET':
        this.emitCode(`
          if (${url}.endsWith('.pdf') && !${url}.startsWith('https')) {
            ${left} = await sys.getPdf({pid: pid, file: ${url}});
          } else {
            let __${left} = null;
            await retry(async (bail) => {
              await ensureTokens();
              __${left} = await sys.getHttp({
                pid: pid,
                file: ${url},
                addressOrHeaders: headers,
                httpUsername,
                httpPs
              });
            }, { retries: 5 });

            ${left} = __${left};
            __${left} = null;
          }
        `);
        break;

      case 'POST':
        this.emitCode(`
          await retry(async (bail) => {
            await ensureTokens();
            __${left} = await sys.postByHttp({
              pid: pid,
              url: ${url},
              data: ${data},
              headers
            });
          }, { retries: 5 });

          ${left} = __${left};
        `);
        break;

      case 'PUT':
        this.emitCode(`
          await retry(async (bail) => {
            await ensureTokens();
            __${left} = await sys.putByHttp({
              pid: pid,
              url: ${url},
              data: ${data},
              headers
            });
          }, { retries: 5 });

          ${left} = __${left};
        `);
        break;
    }
  }

  private generateImageAssignment(left: string, right: ASTNode): void {
    const {operation, args} = right.value;

    switch (operation) {
      case 'BLUR':
        this.emitCode(`
          ${left} = await img.blur({
            pid: pid,
            args: [${args.join(',')}]
          })`
        );
        break;

      case 'SHARPEN':
        this.emitCode(`
          ${left} = await img.sharpen({
            pid: pid,
            args: [${args.join(',')}]
          })`
        );
        break;

      case 'GET IMAGE':
        this.emitCode(`
          ${left} = await img.getImageFromPrompt({
            pid: pid,
            prompt: ${args[0]}
          })`
        );
        break;
    }
  }

  private generateDateAssignment(left: string, right: ASTNode): void {
    const {operation, params} = right.value;

    switch (operation) {
      case 'DATEDIFF':
        this.emitCode(`
          ${left} = await dk.getDateDiff({
            pid: pid,
            date1: ${params.date1},
            date2: ${params.date2},
            mode: ${params.mode}
          })`
        );
        break;

      case 'DATEADD':
        this.emitCode(`
          ${left} = await dk.dateAdd({
            pid: pid,
            date: ${params.date},
            mode: ${params.mode},
            units: ${params.units}
          })`
        );
        break;
    }
  }
  private parseWriteStatement(): ASTNode {
    this.currentToken++; // Skip WRITE

    // Parse the expression to be written
    const expression = this.parseExpression();

    // WRITE is syntactic sugar for PRINT in BASIC
    return {
      type: 'PrintStatement',  // We reuse PrintStatement since WRITE converts to PRINT
      value: expression,
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private parseRemStatement(): ASTNode {
    this.currentToken++; // Skip REM
    
    // Collect all tokens until end of line as comment text
    let commentText = '';
    while (this.currentToken < this.tokens.length && 
           this.tokens[this.currentToken].type !== 'NEWLINE') {
      commentText += this.tokens[this.currentToken].value + ' ';
      this.currentToken++;
    }

    return {
      type: 'CommentStatement',
      value: commentText.trim(),
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private parseCloseStatement(): ASTNode {
    this.currentToken++; // Skip CLOSE

    // Optional file number or identifier
    let fileRef = null;
    if (this.peek().type !== 'NEWLINE') {
      fileRef = this.parseExpression();
    }

    return {
      type: 'CloseStatement',
      value: fileRef,
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private generateCloseCode(ast: ASTNode): void {
    // CLOSE is typically a no-op in the modern context
    // but we might want to handle cleanup of file handles
    if (ast.value) {
      this.emitCode(`// Closing file ${ast.value}`);
    }
  }

  private generateWriteCode(ast: ASTNode): void {
    // WRITE statement gets converted to PRINT
    this.emitCode(`PRINT${this.generateExpression(ast.value)}`);
  }

  private generateRemCode(ast: ASTNode): void {
    // Comments are preserved but as JS-style comments
    this.emitCode(`// ${ast.value}`);
  }

  // Helper method to handle file operations
  private isFileOperation(token: Token): boolean {
    return token.value.startsWith('#') || 
           this.peek(1)?.value === '#' ||
           this.isFileHandle(token);
  }

  private isFileHandle(token: Token): boolean {
    // Check if token represents a file handle
    return token.type === 'WORD' && 
           this.fileHandles.has(token.value);
  }

  // Additional helper for parsing file specifications
  private parseFileSpec(): {
    handle: string;
    mode?: string;
    access?: string;
  } {
    let handle: string;
    let mode: string;
    let access: string;

    if (this.peek().value === '#') {
      this.currentToken++; // Skip #
      handle = this.parseExpression().value;
    } else {
      handle = this.parseExpression().value;
    }

    // Check for optional mode (FOR INPUT/OUTPUT/APPEND)
    if (this.peek().value.toUpperCase() === 'FOR') {
      this.currentToken++; // Skip FOR
      mode = this.peek().value.toUpperCase();
      this.currentToken++; // Skip mode
    }

    // Check for optional access (SHARED/LOCK READ/LOCK WRITE)
    if (this.peek().value.toUpperCase() === 'ACCESS') {
      this.currentToken++; // Skip ACCESS
      access = this.peek().value.toUpperCase();
      this.currentToken++; // Skip access mode
    }

    return { handle, mode, access };
  }

  // Helper for tracking file handles
  private fileHandles = new Set<string>();

  private registerFileHandle(handle: string): void {
    this.fileHandles.add(handle);
  }

  private unregisterFileHandle(handle: string): void {
    this.fileHandles.delete(handle);
  }

  // Extended parseStatement to handle all file operations
  private parseFileStatement(): ASTNode {
    const operation = this.peek().value.toUpperCase();
    this.currentToken++; // Skip operation keyword

    switch (operation) {
      case 'WRITE':
        return this.parseWriteStatement();
      
      case 'PRINT':
        if (this.isFileOperation(this.peek())) {
          return this.parseFilePrintStatement();
        }
        return this.parsePrintStatement();
      
      case 'CLOSE':
        return this.parseCloseStatement();
      
      case 'OPEN':
        return this.parseOpenStatement();
      
      default:
        throw new Error(
          `Unknown file operation ${operation} at line ${this.tokens[this.currentToken].line}`
        );
    }
  }

  private parseFilePrintStatement(): ASTNode {
    const fileSpec = this.parseFileSpec();
    
    if (this.peek().value !== ',') {
      throw new Error('Expected , after file specification');
    }
    this.currentToken++; // Skip ,

    const expressions: ASTNode[] = [];
    
    do {
      expressions.push(this.parseExpression());
      
      if (this.peek().value !== ',') {
        break;
      }
      this.currentToken++; // Skip ,
    } while (true);

    return {
      type: 'FilePrintStatement',
      value: {
        file: fileSpec,
        expressions
      },
      line: this.tokens[this.currentToken].line,
      column: this.tokens[this.currentToken].column
    };
  }

  private generateFormatAssignment(left: string, right: ASTNode): void {
    const {value, format} = right.value;
    
    this.emitCode(`
      ${left} = await dk.format({
        pid: pid,
        value: ${value},
        format: ${format}
      })`
    );
  }

  private generateCardAssignment(left: string, right: ASTNode): void {
    const {doc, data} = right.value;
    
    this.emitCode(`
      ${left} = await dk.card({
        pid: pid,
        args: [${doc}, ${data}]
      })`
    );
  }
  private generateAssignmentCode(ast: ASTNode): void {
    const {left, right} = ast.value;

    switch (right.type) {
      case 'SelectStatement':
        this.generateSelectAssignment(left, right);
        break;

      case 'ChartStatement':
        this.generateChartAssignment(left, right);
        break;

      case 'HttpRequest':
        this.generateHttpAssignment(left, right);
        break;

      default:
        this.emitCode(`${left} = ${this.generateExpression(right)}`);
    }
  }

  // Utility methods
  private emitCode(code: string): void {
    this.output.push(code);
    this.lineMap[this.output.length] = this.line;
  }

  private isNumber(value: any): boolean {
    return !isNaN(parseFloat(value)) && isFinite(value);
  }

  private normalizeQuotes(text: string): string {
    if (!text.trim().startsWith('`') && !text.trim().startsWith("'")) {
      return '`' + text + '`';
    }
    return text;
  }

  private generateParams(names: string[], values: any[]): string {
    let params = '';
    names.forEach((name, i) => {
      const value = values[i];
      params += `"${name}": ${value === undefined ? null : value}${i < names.length - 1 ? ', ' : ''}`;
    });
    return params;
  }

  private peek(offset: number = 0): Token {
    return this.tokens[this.currentToken + offset];
  }

  private generateMetadata(mainName: string): any {
    return {
      name: mainName,
      description: this.systemPromptBuffer || '',
      properties: []
    };
  }

  // Helper method to split params but ignore commas in quotes (from KeywordsExpressions)
  private splitParams(str: string): string[] {
    return str.split(',').reduce(
      (accum: {soFar: string[], isConcatting: boolean}, curr: string) => {
        if (accum.isConcatting) {
          accum.soFar[accum.soFar.length - 1] += ',' + curr;
        } else {
          accum.soFar.push(curr ? curr.trim() : '');
        }
        
        if (curr.split('`').length % 2 === 0) {
          accum.isConcatting = !accum.isConcatting;
        }
        
        return accum;
      },
      { soFar: [], isConcatting: false }
    ).soFar;
  }

  // Converts BASIC conditions to JS conditions
  private convertConditions(input: string): string {
    let result = input.replace(/ +and +/gi, ' && ');
    result = result.replace(/ +or +/gi, ' || ');
    result = result.replace(/ +not +/gi, ' !');
    result = result.replace(/ +<> +/gi, ' !== ');
    result = result.replace(/ += +/gi, ' === ');
    return result;
  }
}

// Usage:
// const compiler = new GBCompiler();
// const result = compiler.compile(source, {
//   filename: 'test.bas',
//   mainName: 'TestProgram',
//   pid: 'process-id'
// });