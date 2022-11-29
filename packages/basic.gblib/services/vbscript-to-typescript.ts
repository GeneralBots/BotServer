// Source: https://github.com/uweg/vbscript-to-typescript
'use strict';

import fs_1 from 'fs';
import path from 'path';

export function convertFile (file) {
  var extension = path.extname(file);
  var withoutExtension = file.substr(0, file.length - extension.length);
  var targetFile = withoutExtension + '.ts';
  var baseName = path.basename(file, extension);
  var content = fs_1.readFileSync(file, 'utf8');
  var result = convert(content, baseName);
  console.log('Writing to "' + targetFile + '"...');
  fs_1.writeFileSync(targetFile, result);
}

export function convert (input, name) {
  var result = convertImports(input, name);
  return result;
}

function convertImports (input, name) {
  var items = [];
  var result = input.replace(/<!-- #include file="(.*?\/)?(.*?).asp" -->/gi, function (input, group1, group2) {
    var path = group1 || './';
    var file = '' + path + group2;
    items.push({ name: group2, path: file });
    return '<%\n' + group2 + '();\n%>';
  });
  result = convertCode(result);
  result = convertExpressions(result);
  result = convertStrings(result);

  for (var _i = 0, items_1 = items; _i < items_1.length; _i++) {
    var item = items_1[_i];
    result = 'import {' + item.name + '} from "' + item.path + '"\n' + result;
  }
  return result;
}

function convertCode (input) {
  var result = input.replace(/<%([^=][\s\S]*?)%>/gi, function (input, group1) {
    var code = group1;
    code = convertComments(code);
    code = convertIfStatements(code);
    code = convertSwitchStatements(code);
    code = convertFunctions(code);
    code = convertForStatements(code);
    code = convertLoops(code);
    code = convertPRec(code);
    code = convertPLan(code);
    return '<%' + code + '%>';
  });
  return result;
}

function convertExpressions (input) {
  var result = input.replace(/<%=([\s\S]*?)%>/gi, function (input, group1) {
    var content = convertPRec(group1);
    content = convertPLan(content);
    return '${' + content + '}';
  });
  return result;
}

function convertStrings (input) {
  var result = input.replace(/%>([\s\S]+?)<%/gi, '\nResponse.Write(`$1`);\n');
  // Entire document is a string
  if (result.indexOf('<%') === -1) {
    result = 'Response.Write(`' + result + '`);';
  }
  // Start of the document is a string
  var firstIndex = result.indexOf('<%');
  if (firstIndex > 0) {
    result = 'Response.Write(`' + result.substr(0, firstIndex) + '`);\n' + result.substring(firstIndex + 2);
  }
  result = result.replace(/%>$/, '');
  // End of the document is a string
  var lastIndex = result.lastIndexOf('%>');
  if (lastIndex > -1 && lastIndex < result.length - 2) {
    result = result.substr(0, lastIndex) + '\nResponse.Write(`' + result.substr(lastIndex + 3) + '`);';
  }
  result = result.replace(/^<%/, '');
  return result;
}

function convertComments (input) {
  var result = '';
  var splitted = input.split(/(".*")/gim);
  for (var _i = 0, splitted_1 = splitted; _i < splitted_1.length; _i++) {
    var part = splitted_1[_i];
    if (part.indexOf('"') === 0) {
      result += part;
    } else {
      result += part.replace(/'/gi, '//');
    }
  }
  return result;
}

function convertIfStatements (input) {
  var result = input.replace(/if +(.*?) +then/gi, function (input, group1) {
    var condition = convertConditions(group1);
    return '\nif (' + condition + ') {\n';
  });
  result = result.replace(/end if/gi, '\n}\n');
  result = result.replace(/else(?!{)/gi, '\n}\nelse {\n');
  return result;
}

function convertSwitchStatements (input) {
  var result = input.replace(/select case +(.*)/gi, '\nswitch ($1) {\n');
  result = result.replace(/end select/gi, '\n}\n');
  return result;
}

function convertFunctions (input) {
  var result = input.replace(/function +(.*)\((.*)\)/gi, '\n$1 = ($2) => {\n');
  result = result.replace(/end function/gi, '\n}\n');
  return result;
}

function convertForStatements (input) {
  var result = input.replace(/for +(.*to.*)/gi, '\nfor ($1) {\n');
  result = result.replace(/^ *next *$/gim, '}\n');
  return result;
}

function convertConditions (input) {
  var result = input.replace(/ +and +/gi, ' && ');
  result = result.replace(/ +or +/gi, ' || ');
  result = result.replace(/ +<> +/gi, ' !== ');
  result = result.replace(/ += +/gi, ' === ');
  return result;
}

function convertLoops (input) {
  var result = input.replace(/do while +(.*)/gi, function (input, group1) {
    var condition = convertConditions(group1);
    return '\nwhile (' + condition + ') {\n';
  });

  result = result.replace(/^ *loop *$/gim, '}\n');
  return result;
}

function convertPRec (input) {
  var result = input.replace(/(p_rec\("\S+?"\))/gi, '$1.Value');
  return result;
}

function convertPLan (input) {
  var result = input.replace(/(l_\S+?)\(p_lan\)/gi, '$1[p_lan]');
  return result;
}
