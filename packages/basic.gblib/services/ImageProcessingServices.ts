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

import path from 'path';
import { GBLog, GBMinInstance } from 'botlib';
import { DialogKeywords } from './DialogKeywords.js';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import urlJoin from 'url-join';
import { GBServer } from '../../../src/app.js';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';
import { GBUtil } from '../../../src/util.js';
import fs from 'fs/promises';
import { AzureOpenAI } from 'openai';
import { OpenAIClient } from '@langchain/openai';

/**
 * Image processing services of conversation to be called by BASIC.
 */
export class ImageProcessingServices {
  /**
   * Sharpen the image.
   *
   * @example file = SHARPEN file
   */
  public async sharpen({ pid, file: file }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `Image Processing SHARPEN ${file}.`);

    const gbfile = DialogKeywords.getFileByHandle(file);

    // TODO: sharp.
    return;
  }

  /**
   * SET ORIENTATION VERTICAL
   *
   * file = MERGE file1, file2, file3
   */
  public async mergeImage({ pid, files }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);

    let paths = [];
    await CollectionUtil.asyncForEach(files, async file => {
      const gbfile = DialogKeywords.getFileByHandle(file);
      paths.push(gbfile.path);
    });

    const botId = min.instance.botId;
    const packagePath = GBUtil.getGBAIPath(min.botId);
    // TODO: const img = await joinImages(paths);
    const localName = path.join(
      'work',
      packagePath,
      'cache',
      `img-mrg${GBAdminService.getRndReadableIdentifier()}.png`
    );
    const url = urlJoin(GBServer.globals.publicAddress, min.botId, 'cache', path.basename(localName));
    //    img.toFile(localName);

    return { localName: localName, url: url, data: null };
  }

  /**
   * Sharpen the image.
   *
   * @example file = BLUR file
   */
  public async blur({ pid, file: file }) {
    const { min, user } = await DialogKeywords.getProcessInfo(pid);
    GBLogEx.info(min, `Image Processing SHARPEN ${file}.`);

    const gbfile = DialogKeywords.getFileByHandle(file);
    return;
  }

  public async getImageFromPrompt({ pid, prompt }) {
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);

    GBLogEx.info(min, `DALL-E: ${prompt}.`);

    const azureOpenAIKey = await min.core.getParam(min.instance, 'Azure Open AI Key', null, true);
    const azureOpenAIEndpoint = await min.core.getParam(min.instance, 'Azure Open AI Endpoint', null, true);
    const azureOpenAIVersion = await (min.core as any)['getParam'](min.instance, 'Azure Open AI Version', null, true);
    const azureOpenAIImageModel = await (min.core as any)['getParam'](min.instance, 'Azure Open AI Image Model', null, true);
    

    if (azureOpenAIKey) {
      // Initialize the Azure OpenAI client

      const client = new AzureOpenAI({
        endpoint: azureOpenAIEndpoint,
        deployment: azureOpenAIImageModel,
        apiVersion: azureOpenAIVersion,
        apiKey: azureOpenAIKey
      });
      
      // Make a request to the image generation endpoint
      
      const response = await client.images.generate({
        model: '',
        prompt: prompt,
        n: 1,
        size: '1024x1024'
      });

      const gbaiName = GBUtil.getGBAIPath(min.botId);
      const localName = path.join('work', gbaiName, 'cache', `DALL-E${GBAdminService.getRndReadableIdentifier()}.png`);

      const url = response.data[0].url;
      const res = await fetch(url);
      let buf: any = Buffer.from(await res.arrayBuffer());
      await fs.writeFile(localName, buf, { encoding: null });

      GBLogEx.info(min, `DALL-E: ${url} - ${response.data[0].revised_prompt}.`);

      return { localName, url };
    }
  }

  public async getCaptionForImage({ pid, imageUrl }) {
    const { min, user, params } = await DialogKeywords.getProcessInfo(pid);

    const azureOpenAIKey = await min.core.getParam(min.instance, 'Azure Open AI Key', null);
    const azureOpenAITextModel = 'gpt-4'; // Specify GPT-4 model here
    const azureOpenAIEndpoint = await min.core.getParam(min.instance, 'Azure Open AI Endpoint', null);
    const azureOpenAIVersion = await (min.core as any)['getParam'](min.instance, 'Azure Open AI Version', null, true);

    if (azureOpenAIKey && azureOpenAITextModel && imageUrl) {
      const client = new AzureOpenAI({
        apiVersion: azureOpenAIVersion,
        apiKey: azureOpenAIKey,
        baseURL: azureOpenAIEndpoint
      });

      const prompt = `Provide a descriptive caption for the image at the following URL: ${imageUrl}`;

      const response = await client.completions.create({

        model: azureOpenAITextModel,
        prompt: prompt,
        max_tokens: 50
      });

      const caption = response['data'].choices[0].text.trim();
      GBLogEx.info(min, `Generated caption: ${caption}`);

      return { caption };
    }
  }
}
