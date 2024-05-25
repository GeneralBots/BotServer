/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__,\| (˅) |( (_) )  |
|   | ,__/'(_)  `\__,_)`\__  |(_) (_) (_)`\__,_)`\__)(_)(____/(_) (_)`\___/'  |
|   | |                ( )_) |                                                |
|   (_)                 \___/'                                                |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.         |
| Licensed under the AGPL-3.0.                                                |
|                                                                             |
| According to our dual licensing model,this program can be used either      |
| under the terms of the GNU Affero General Public License,version 3,       |
| or under a proprietary license.                                             |
|                                                                             |
| The texts of the GNU Affero General Public License with an additional       |
| permission and of our proprietary license can be found at and               |
| in the LICENSE file you have received along with this program.              |
|                                                                             |
| This program is distributed in the hope that it will be useful,            |
| but WITHOUT ANY WARRANTY,without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of pragmatismo.cloud.             |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights,title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

'use strict';

// import { GBMinInstance } from 'botlib';
// import {DallEAPIWrapper} from '@langchain/openai';

// /**
//  * Image processing services of conversation to be called by BASIC.
//  */
// export class ImageServices {
//   public async getImageFromDescription(min: GBMinInstance, text: string): Promise<string> {
//     const azureOpenAIKey = await min.core.getParam(min.instance, 'Azure Open AI Key', null);
//     const azureOpenAIImageModel = await min.core.getParam(min.instance, 'Azure Open Image Model', null);
//     const azureOpenAIVersion = await min.core.getParam(min.instance, 'Azure Open AI Version', null);
//     const azureOpenAIApiInstanceName = await min.core.getParam(min.instance, 'Azure Open AI Instance', null);

//     if (azureOpenAIKey) {
//       const tool = new DallEAPIWrapper({
//         n: 1,
//         model: 'dall-e-3',
//         apiKey: azureOpenAIKey
//       });

//       const imageURL = await tool.invoke('a painting of a cat');

//       return imageURL;
//     }
//   }
// }
