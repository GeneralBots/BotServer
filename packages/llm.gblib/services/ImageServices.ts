/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.          |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.              |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

'use strict';

import { GBMinInstance } from 'botlib';
import OpenAI from 'openai';

import { AzureKeyCredential } from '@azure/core-auth';
import { DialogKeywords } from '../../basic.gblib/services/DialogKeywords';
import Path from 'path';
import { GBServer } from '../../../src/app.js';
import Fs from 'fs';
import urlJoin from 'url-join';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx';

/**
 * Image processing services of conversation to be called by BASIC.
 */
export class ImageServices {
  public async getImageFromPrompt({ pid, prompt }) {
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);

    const azureOpenAIKey = await min.core.getParam(min.instance, 'Azure Open AI Key', null);
    const azureOpenAIImageModel = await min.core.getParam(min.instance, 'Azure Open Image Model', null);
    const azureOpenAIEndpoint = await min.core.getParam(min.instance, 'Azure Open AI Endpoint', null);

    if (azureOpenAIKey) {
      // Initialize the Azure OpenAI client

      const client = new OpenAI({ apiKey: azureOpenAIKey, baseURL: azureOpenAIEndpoint });

      // Make a request to the image generation endpoint

      const response = await client.images.generate({
        prompt: prompt,
        n: 1,
        size: '1024x1024'
      });

      const gbaiName = DialogKeywords.getGBAIPath(min.botId);
      const localName = Path.join('work', gbaiName, 'cache', `DALL-E${GBAdminService.getRndReadableIdentifier()}.png`);

      const url = response.data[0].url;
      const res = await fetch(url);
      let buf: any = Buffer.from(await res.arrayBuffer());
      Fs.writeFileSync(localName, buf, { encoding: null });

      GBLogEx.info(min, `DALL-E image generated at ${url}.`);

      return { localName, url };
    }
  }
}
