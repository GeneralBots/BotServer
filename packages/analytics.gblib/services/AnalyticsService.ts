/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.          |
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
| "General Bots" is a registered trademark of pragmatismo.com.br.              |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

/**
 * @fileoverview General Bots server core.
 */

import { FindOptions } from 'sequelize/types';
import { GBServer } from '../../../src/app.js';
import { GuaribasUser } from '../../security.gbapp/models/index.js';
import { GuaribasConversation, GuaribasConversationMessage } from '../models/index.js';

/**
 * Base services for Bot Analytics.
 */
export class AnalyticsService {
  public async createConversation(user: GuaribasUser): Promise<GuaribasConversation> {
    const conversation = new GuaribasConversation();
    conversation.startedBy = user;
    conversation.startedByUserId = user.userId;
    conversation.instanceId = user.instanceId;

    return await conversation.save();
  }

  public async updateConversationSuggestion(
    instanceId: number,
    conversationId: string,
    feedback: string,
    locale: string
  ): Promise<number> {
    const minBoot = GBServer.globals.minBoot as any;
    return 0;
  }

  public async createMessage(
    instanceId: number,
    conversationId: number,
    userId: number,
    content: string
  ): Promise<GuaribasConversationMessage> {
    const message = GuaribasConversationMessage.build();
    message.content = typeof content === 'object' ? JSON.stringify(content) : content;
    message.instanceId = instanceId;
    message.userId = userId;
    message.conversationId = conversationId;

    return await message.save();
  }
}
