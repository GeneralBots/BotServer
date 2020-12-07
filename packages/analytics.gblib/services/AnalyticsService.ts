/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
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

import { GuaribasUser } from '../../security.gbapp/models';
import { GuaribasConversation, GuaribasConversationMessage } from '../models';
import { GBServer } from '../../../src/app';
import { AzureText } from 'pragmatismo-io-framework';

/**
 * Base services for Bot Analytics.
 */
export class AnalyticsService {

  public async createConversation(
    user: GuaribasUser
  ): Promise<GuaribasConversation> {
    const conversation = new GuaribasConversation();
    conversation.startedBy = user;
    conversation.startedByUserId = user.userId;
    conversation.instanceId = user.instanceId;

    return await conversation.save();
  }

  public async updateConversationSuggestion(instanceId: number, conversationId: string,  feedback: string, locale: string): Promise<number> {
    
    const minBoot = GBServer.globals.minBoot as any;
    const rate = await AzureText.getSentiment(
      minBoot.instance.textAnalyticsKey ? minBoot.instance.textAnalyticsKey : minBoot.instance.textAnalyticsKey,
      minBoot.instance.textAnalyticsEndpoint ? minBoot.instance.textAnalyticsEndpoint : minBoot.instance.textAnalyticsEndpoint,
      locale,
      feedback
    );
    
    const options = { where: { } };
    options.where = { conversationId: conversationId,  instanceId: instanceId };
    const item = await GuaribasConversation.findOne(options);

    item.feedback = feedback;
    item.rate = rate;
    item.rateDate = new Date();
    await item.save();

    return rate;

  }


  public async createMessage(
    instanceId: number,
    conversation: GuaribasConversation,
    userId: number,
    content: string
  ): Promise<GuaribasConversationMessage> {

    const message = GuaribasConversationMessage.build();
    message.content = content;
    message.instanceId = instanceId;
    message.userId = userId;
    message.conversationId = conversation.conversationId;

    return await message.save();
  }
}
