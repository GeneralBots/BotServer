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

import { FindOptions, NonNullFindOptions } from 'sequelize/types';
import { GuaribasQuestion } from '../../../packages/kb.gbapp/models/index.js';
import { GuaribasConversation } from '../../analytics.gblib/models/index.js';
import { GuaribasQuestionAlternate } from '../models/index.js';

/**
 * Customer Satisfaction Service Layer.
 */
export class CSService {
  public async getQuestionFromAlternateText (instanceId: number, text: string): Promise<GuaribasQuestion> {
    const questionAlternate = await GuaribasQuestionAlternate.findOne({
      where: {
        instanceId: instanceId,
        questionTyped: text
      }
    });

    let question: GuaribasQuestion = null;

    if (questionAlternate !== null) {
      question = await GuaribasQuestion.findOne({
        where: {
          instanceId: instanceId,
          questionId: questionAlternate.questionTyped
        }
      });
    }

    return question;
  }

  public async insertQuestionAlternate (
    instanceId: number,
    questionTyped: string,
    questionText: string
  ): Promise<GuaribasQuestionAlternate> {
    return await GuaribasQuestionAlternate.create(<GuaribasQuestionAlternate>{
      questionTyped: questionTyped,
      questionText: questionText
    });
  }

  public async updateConversationRate (
    conversation: GuaribasConversation,
    rate: number
  ): Promise<GuaribasConversation> {
    conversation.rate = rate;

    return conversation.save();
  }
}
