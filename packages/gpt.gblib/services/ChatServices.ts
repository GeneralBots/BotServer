/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.         |
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
| "General Bots" is a registered trademark of pragmatismo.com.br.             |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

'use strict';

import { RunnableSequence } from "@langchain/core/runnables";
import { ChatOpenAI } from "@langchain/openai";
import { GBMinInstance } from 'botlib';
import * as Fs from 'fs';
import { formatXml } from "langchain/agents/format_scratchpad/xml";
import { jsonSchemaToZod } from "json-schema-to-zod";
import { renderTextDescription } from "langchain/tools/render";

import { AgentExecutor, AgentStep } from "langchain/agents";
import { BufferWindowMemory } from 'langchain/memory';
import { AIMessagePromptTemplate, ChatPromptTemplate, HumanMessagePromptTemplate, MessagesPlaceholder } from '@langchain/core/prompts';
import { Tool } from "langchain/tools";
import { HNSWLib } from '@langchain/community/vectorstores/hnswlib';
import Path from 'path';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { DialogKeywords } from '../../basic.gblib/services/DialogKeywords.js';
import { GBVMService } from '../../basic.gblib/services/GBVMService.js';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import { GuaribasSubject } from '../../kb.gbapp/models/index.js';
import { XMLAgentOutputParser } from "langchain/agents/xml/output_parser";
import { StringOutputParser } from "@langchain/core/output_parsers";
import { convertToOpenAITool } from "@langchain/core/utils/function_calling";



export class ChatServices {

  private static async getRelevantContext(
    vectorStore: HNSWLib,
    sanitizedQuestion: string,
    numDocuments: number
  ): Promise<string> {
    const documents = await vectorStore.similaritySearch(sanitizedQuestion, numDocuments);
    return documents
      .map((doc) => doc.pageContent)
      .join(', ')
      .trim()
      .replaceAll('\n', ' ');
  }

  /**
   * Generate text
   *
   * CONTINUE keword.
   *
   * result = CONTINUE text
   *
   */
  public static async continue(min: GBMinInstance, text: string, chatId) {

  }


  public static async answerByGPT(min: GBMinInstance, pid,
    query: string,
    searchScore: number,
    subjects: GuaribasSubject[]
  ) {

    if (!process.env.OPENAI_API_KEY) {
      return { answer: undefined, questionId: 0 };
    }

    const contentLocale = min.core.getParam(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    let tools = await ChatServices.getTools(min);
    let toolsAsText = ChatServices.getToolsAsText(tools);

    const memory = new BufferWindowMemory({
      returnMessages: true,
      memoryKey: 'chat_history',
      inputKey: 'input',
      k: 2,
    });

    const model = new ChatOpenAI({
      openAIApiKey: process.env.OPENAI_API_KEY,
      modelName: "gpt-3.5-turbo-0125",
      temperature: 0,
    });
    
    const contextVectorStore = min['vectorStore'];

    let promptTemplate = `Answer in ${contentLocale}. 
    You have access to the context (RELEVANTDOCS) provided by the user.
    
    When answering think about whether the question in RELEVANTDOCS, but never mention
    to user about the source.
    Don’t justify your answers. Don't refer to yourself in any of the created content.
    Don´t prefix RESPONSE: when answering the user.
    RELEVANTDOCS: {context}

    QUESTION: """{input}"""

    You have the following tools that you can invoke based on the user inquiry. 
    Tools: 

    ${toolsAsText}

    `;


    const toolMap: Record<string, any> = {
      multiply: ()=>{},
    };

    const modelWithTools = model.bind({
      tools: tools.map(convertToOpenAITool),
    });

    const questionGeneratorTemplate = ChatPromptTemplate.fromMessages([
      AIMessagePromptTemplate.fromTemplate(
        "Given the following conversation about a codebase and a follow up question, rephrase the follow up question to be a standalone question."
      ),
      new MessagesPlaceholder("chat_history"),
      AIMessagePromptTemplate.fromTemplate(`Follow Up Input: {question} Standalone question:`),
    ]);

    const combineDocumentsPrompt = ChatPromptTemplate.fromMessages([
      AIMessagePromptTemplate.fromTemplate(
        "Use the following pieces of context to answer the question at the end. If you don't know the answer, just say that you don't know, don't try to make up an answer.\n\n{context}\n\n"
      ),
      new MessagesPlaceholder("chat_history"),
      HumanMessagePromptTemplate.fromTemplate("Question: {question}"),
    ]);

    const combineDocumentsChain = RunnableSequence.from([
      {
        question: (output: string) => output,
        chat_history: async () => {
          const { chat_history } = await memory.loadMemoryVariables({});
          return chat_history;
        },
        context: async (output: string) => {

          return await this.getRelevantContext(contextVectorStore, output, 1);
        },
      },
      combineDocumentsPrompt,
      modelWithTools,
      new StringOutputParser(),
    ]);

    const conversationalQaChain = RunnableSequence.from([
      {
        question: (i: { question: string }) => i.question,
        chat_history: async () => {
          const { chat_history } = await memory.loadMemoryVariables({});
          return chat_history;
        },
      },
      questionGeneratorTemplate,
      modelWithTools,
      new StringOutputParser(),
      combineDocumentsChain,
    ]);

    const question = "How can I initialize a ReAct agent?";
    let result = await conversationalQaChain.invoke({
      question,
    });

    return { answer: result.toString() , questionId: 0 };

  }


  private static getToolsAsText(tools) {
    return Object.keys(tools)
      .map((toolname) => `${tools[toolname].function.name}: ${tools[toolname].function.description}`)
      .join("\n");
  }

  private static async getTools(min: GBMinInstance) {
    let functions = [];

    // Adds .gbdialog as functions if any to GPT Functions.
    await CollectionUtil.asyncForEach(Object.keys(min.scriptMap), async (script) => {
      const path = DialogKeywords.getGBAIPath(min.botId, "gbdialog", null);
      const functionJSON = Path.join('work', path, `${script}.json`);

      if (Fs.existsSync(functionJSON)) {
        const func = JSON.parse(Fs.readFileSync(functionJSON, 'utf8'));
        func.schema =  jsonSchemaToZod(func.properties, { module: "esm" });
          func.func = async ()=>{
              const name = '';
              const pid = 1;
              const text = ''; // TODO:
              const result =  await GBVMService.callVM(name, min, false, pid,false, [text]);

          }

        functions.push(func);
      }

    });
    return functions;
  }
}
