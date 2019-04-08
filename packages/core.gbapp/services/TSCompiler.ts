/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| (˅) |( (_) )  |
|   | ,__/'(_)  `\__,_)`\__  |(_) (_) (_)`\__,_)`\__)(_)(____/(_) (_)`\___/'  |
|   | |                ( )_) |                                                |
|   (_)                 \___/'                                                |
|                                                                             |
| General Bots Copyright (c) Pragmatismo.io. All rights reserved.             |
| Licensed under the AGPL-3.0.                                                |
|                                                                             |
| According to our dual licensing model, this program can be used either      |
| under the terms of the GNU Affero General Public License, version 3,        |
| or under a proprietary license.                                             |
|                                                                             |
| The texts of the GNU Affero General Public License with an additional       |
| permission and of our proprietary license can be found at and               |
| in the LICENSE file you have received along with this program.              |
|                                                                             |
| This program is distributed in the hope that it will be useful,             |
| but WITHOUT ANY WARRANTY, without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

/**
 * @fileoverview General Bots server core.
 */

'use strict';

import { GBLog } from 'botlib';
import * as ts from 'typescript';

/**
 * Wrapper for a TypeScript compiler.
 */
export class TSCompiler {

  private static shouldIgnoreError(diagnostic) {
    const message = ts.flattenDiagnosticMessageText(diagnostic.messageText, '\n');

    if (message.indexOf('Cannot find name') >= 0 || message.indexOf('Cannot use imports') >= 0) {
      return true;
    }

    return false;
  }

  public compile(
    fileNames: string[],
    options: ts.CompilerOptions = {
      noStrictGenericChecks: true,
      noImplicitUseStrict: true,
      noEmitOnError: false,
      noImplicitAny: true,
      target: ts.ScriptTarget.ES5,
      module: ts.ModuleKind.None,
      moduleResolution: ts.ModuleResolutionKind.Classic,
      noEmitHelpers: true,
      maxNodeModuleJsDepth: 0,
      esModuleInterop: false
    }
  ) {
    const program = ts.createProgram(fileNames, options);
    const emitResult = program.emit();

    const allDiagnostics = ts.getPreEmitDiagnostics(program).concat(emitResult.diagnostics);

    allDiagnostics.forEach(diagnostic => {
      if (!TSCompiler.shouldIgnoreError(diagnostic)) {
        const message = ts.flattenDiagnosticMessageText(diagnostic.messageText, '\n');

        if (diagnostic.file !== undefined) {
          const { line, character } = diagnostic.file.getLineAndCharacterOfPosition(diagnostic.start);
          GBLog.error(`${diagnostic.file.fileName} (${line + 1},${character + 1}): ${message}`);
        } else {
          GBLog.error(`${message}`);
        }
      }
    });

    return emitResult;
  }

}
