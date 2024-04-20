/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.         |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.             |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

/**
 * @fileoverview General Bots local utility.
 */

'use strict';


export class GBUtil {

  public static repeat (chr, count) {
    var str = "";
    for (var x = 0; x < count; x++) { str += chr };
    return str;
  }

  public static padL  (value, width, pad) {
    if (!width || width < 1)
      return value;

    if (!pad) pad = " ";
    var length = width - value.length
    if (length < 1) return value.substr(0, width);

    return (GBUtil.repeat(pad, length) + value).substr(0, width);
  }
  public static padR  (value, width, pad) {
    if (!width || width < 1)
      return value;

    if (!pad) pad = " ";
    var length = width - value.length
    if (length < 1) value.substr(0, width);

    return (value + GBUtil.repeat(pad, length)).substr(0, width);
  }


  public static sleep(ms) {
    return new Promise(resolve => {
      setTimeout(resolve, ms);
    });
  };

  public static caseInsensitive(listOrRow) {

    if (!listOrRow) {

      return listOrRow;
    };

    const lowercase = (oldKey) => typeof oldKey === 'string' ? oldKey.toLowerCase() : oldKey;

    const createCaseInsensitiveProxy = (obj) => {
      const propertiesMap = new Map(Object.keys(obj).map(propKey => [lowercase(propKey), obj[propKey]]));
      const caseInsensitiveGetHandler = {
        get: (target, property) => propertiesMap.get(lowercase(property))
      };
      return new Proxy(obj, caseInsensitiveGetHandler);
    };

    if (listOrRow.length) {
      return listOrRow.map(row => createCaseInsensitiveProxy(row));
    } else {
      return createCaseInsensitiveProxy(listOrRow);
    }
  };

}