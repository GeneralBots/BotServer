"use strict";
exports.__esModule = true;
var fs_1 = require("fs");
var path = require("path");
function convertFile(file) {
    var extension = path.extname(file);
    var withoutExtension = file.substr(0, file.length - extension.length);
    var targetFile = withoutExtension + ".ts";
    var baseName = path.basename(file, extension);
    var content = fs_1.readFileSync(file, 'utf8');
    var result = convert(content, baseName);
    console.log("Writing to \"" + targetFile + "\"...");
    fs_1.writeFileSync(targetFile, result);
}
exports.convertFile = convertFile;
function convert(input, name) {
    var result = convertImports(input, name);
    return result;
}
exports.convert = convert;
function convertImports(input, name) {
    var items = [];
    var result = input.replace(/<!-- #include file="(.*?\/)?(.*?).asp" -->/g, function (input, group1, group2) {
        var path = group1 || './';
        var file = "" + path + group2;
        items.push({ name: group2, path: file });
        return "<%\n" + group2 + "();\n%>";
    });
    result = convertCode(result);
    result = convertExpressions(result);
    result = convertStrings(result);
    result = "\nexport function " + name + "() {\n" + result + "\n}";
    for (var _i = 0, items_1 = items; _i < items_1.length; _i++) {
        var item = items_1[_i];
        result = "import {" + item.name + "} from \"" + item.path + "\"\n" + result;
    }
    return result;
}
exports.convertImports = convertImports;
function convertCode(input) {
    var result = input.replace(/<%([^=][\s\S]*?)%>/g, function (input, group1) {
        var code = group1;
        code = convertComments(code);
        code = convertIfStatements(code);
        code = convertSwitchStatements(code);
        code = convertFunctions(code);
        code = convertForStatements(code);
        code = convertLoops(code);
        code = convertPRec(code);
        code = convertPLan(code);
        return "<%" + code + "%>";
    });
    return result;
}
exports.convertCode = convertCode;
function convertExpressions(input) {
    var result = input.replace(/<%=([\s\S]*?)%>/g, function (input, group1) {
        var content = convertPRec(group1);
        content = convertPLan(content);
        return "${" + content + "}";
    });
    return result;
}
exports.convertExpressions = convertExpressions;
function convertStrings(input) {
    var result = input.replace(/%>([\s\S]+?)<%/g, "\nResponse.Write(`$1`);\n");
    // Entire document is a string
    if (result.indexOf("<%") === -1) {
        result = "Response.Write(`" + result + "`);";
    }
    // Start of the document is a string
    var firstIndex = result.indexOf("<%");
    if (firstIndex > 0) {
        result = "Response.Write(`" + result.substr(0, firstIndex) + "`);\n" + result.substring(firstIndex + 2);
    }
    result = result.replace(/%>$/, "");
    // End of the document is a string
    var lastIndex = result.lastIndexOf("%>");
    if (lastIndex > -1 && lastIndex < result.length - 2) {
        result = result.substr(0, lastIndex) + "\nResponse.Write(`" + result.substr(lastIndex + 3) + "`);";
    }
    result = result.replace(/^<%/, "");
    return result;
}
exports.convertStrings = convertStrings;
function convertComments(input) {
    var result = '';
    var splitted = input.split(/(".*")/gm);
    for (var _i = 0, splitted_1 = splitted; _i < splitted_1.length; _i++) {
        var part = splitted_1[_i];
        if (part.indexOf("\"") === 0) {
            result += part;
        }
        else {
            result += part.replace(/'/g, "//");
        }
    }
    return result;
}
exports.convertComments = convertComments;
function convertIfStatements(input) {
    var result = input.replace(/if +(.*?) +then/g, function (input, group1) {
        var condition = convertConditions(group1);
        return "\nif (" + condition + ") {\n";
    });
    result = result.replace(/end if/g, "\n}\n");
    result = result.replace(/else(?!{)/g, "\n}\nelse {\n");
    return result;
}
exports.convertIfStatements = convertIfStatements;
function convertSwitchStatements(input) {
    var result = input.replace(/select case +(.*)/g, "\nswitch ($1) {\n");
    result = result.replace(/end select/g, "\n}\n");
    return result;
}
exports.convertSwitchStatements = convertSwitchStatements;
function convertFunctions(input) {
    var result = input.replace(/function +(.*)\((.*)\)/g, "\n$1 = ($2) => {\n");
    result = result.replace(/end function/g, "\n}\n");
    return result;
}
exports.convertFunctions = convertFunctions;
function convertForStatements(input) {
    var result = input.replace(/for +(.*to.*)/g, "\nfor ($1) {\n");
    result = result.replace(/^ *next *$/gm, "}\n");
    return result;
}
exports.convertForStatements = convertForStatements;
function convertConditions(input) {
    var result = input.replace(/ +and +/g, " && ");
    result = result.replace(/ +or +/g, " || ");
    result = result.replace(/ +<> +/g, " !== ");
    result = result.replace(/ += +/g, " === ");
    return result;
}
exports.convertConditions = convertConditions;
function convertLoops(input) {
    var result = input.replace(/do while +(.*)/g, function (input, group1) {
        var condition = convertConditions(group1);
        return "\nwhile (" + condition + ") {\n";
    });
    
    result = result.replace(/^ *loop *$/gm, "}\n");
    return result;
}
exports.convertLoops = convertLoops;
function convertPRec(input) {
    var result = input.replace(/(p_rec\("\S+?"\))/g, "$1.Value");
    return result;
}
exports.convertPRec = convertPRec;
function convertPLan(input) {
    var result = input.replace(/(l_\S+?)\(p_lan\)/g, "$1[p_lan]");
    return result;
}
exports.convertPLan = convertPLan;
