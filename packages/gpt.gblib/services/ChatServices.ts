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

import { HNSWLib } from '@langchain/community/vectorstores/hnswlib';
import { StringOutputParser } from "@langchain/core/output_parsers";
import { AIMessagePromptTemplate, ChatPromptTemplate, HumanMessagePromptTemplate, MessagesPlaceholder } from '@langchain/core/prompts';
import { RunnableSequence } from "@langchain/core/runnables";
import { convertToOpenAITool } from "@langchain/core/utils/function_calling";
import { ChatOpenAI } from "@langchain/openai";
import { GBLog, GBMinInstance } from 'botlib';
import * as Fs from 'fs';
import { jsonSchemaToZod } from "json-schema-to-zod";
import { BufferWindowMemory } from 'langchain/memory';
import Path from 'path';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { DialogKeywords } from '../../basic.gblib/services/DialogKeywords.js';
import { GBVMService } from '../../basic.gblib/services/GBVMService.js';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import { GuaribasSubject } from '../../kb.gbapp/models/index.js';
import { z } from "zod";
import { DynamicStructuredTool } from "@langchain/core/tools";
import { JsonOutputToolsParser } from "langchain/output_parsers";
import {
  RunnableLambda,
  RunnablePassthrough,
} from "@langchain/core/runnables";
import {
  CombiningOutputParser,
} from "langchain/output_parsers";


import {
  BaseLLMOutputParser,
  OutputParserException,
} from "@langchain/core/output_parsers";
import { ChatGeneration, Generation } from "@langchain/core/outputs";

export interface CustomOutputParserFields { }

// This can be more generic, like Record<string, string>
export type ExpectedOutput = {
  greeting: string;
};

function isChatGeneration(
  llmOutput: ChatGeneration | Generation
): llmOutput is ChatGeneration {
  return "message" in llmOutput;
}

export class CustomLLMOutputParser extends BaseLLMOutputParser<ExpectedOutput> {
  lc_namespace = ["langchain", "output_parsers"];

  constructor(fields?: CustomOutputParserFields) {
    super(fields);
  }

  async parseResult(
    llmOutputs: ChatGeneration[] | Generation[]
  ): Promise<ExpectedOutput> {
    if (!llmOutputs.length) {
      throw new OutputParserException(
        "Output parser did not receive any generations."
      );
    }
    let parsedOutput;


    if (isChatGeneration(llmOutputs[0])) {
      parsedOutput = llmOutputs[0].message.content;
    } else {
      parsedOutput = llmOutputs[0].text;
    }
    let parsedText;
    parsedText = parsedOutput;
    return parsedText;
  }
}

export class ChatServices {

  private static async getRelevantContext(
    vectorStore: HNSWLib,
    sanitizedQuestion: string,
    numDocuments: number
  ): Promise<string> {
    if (sanitizedQuestion === '') {
      return '';
    }

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
  public static async continue(min: GBMinInstance, question: string, chatId) {

  }


  public static async answerByGPT(min: GBMinInstance, user, pid,
    question: string,
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

    const toolMap: Record<string, any> = {
      multiply: tools[0]
    };

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

    const context = min['vectorStore'];

    const modelWithTools = model.bind({
      tools: tools.map(convertToOpenAITool),
      tool_choice: {
        type: "function",
        function: { name: "multiply" },
      },
    });

    // Function for dynamically constructing the end of the chain based on the model-selected tool.
    const callSelectedTool = RunnableLambda.from(
      (toolInvocation: Record<string, any>) => {
        const selectedTool = toolMap[toolInvocation.type];
        if (!selectedTool) {
          throw new Error(
            `No matching tool available for requested type "${toolInvocation.type}".`
          );
        }
        const toolCallChain = RunnableSequence.from([
          (toolInvocation) => toolInvocation.args,
          selectedTool,
        ]);
        // We use `RunnablePassthrough.assign` here to return the intermediate `toolInvocation` params
        // as well, but you can omit if you only care about the answer.
        return RunnablePassthrough.assign({
          output: toolCallChain,
        });
      },

    );

    const questionGeneratorTemplate = ChatPromptTemplate.fromMessages([
      AIMessagePromptTemplate.fromTemplate(
        "Given the following conversation about a codebase and a follow up question, rephrase the follow up question to be a standalone question."
      ),
      new MessagesPlaceholder("chat_history"),
      AIMessagePromptTemplate.fromTemplate(`Follow Up Input: {question}
    Standalone question:`),
    ]);

    const combineDocumentsPrompt = ChatPromptTemplate.fromMessages([
      AIMessagePromptTemplate.fromTemplate(
        `Use the following pieces of context to answer the question at the end. 
        If you don't know the answer, just say that you don't know, don't try to make up an answer.
        \n\n{context}\n\n
        You have the following tools to call: 
        ${toolsAsText}`
        
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
          return await ChatServices.getRelevantContext(context, output, 1);
          
        },
      },
      combineDocumentsPrompt,
      modelWithTools,
      new CustomLLMOutputParser(),
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
    
    

    const systemPrompt = user['systemPrompt'];

    let result = await conversationalQaChain.invoke({
      question,
    });
    // await memory.saveContext(
    //   {
    //     input: query,
    //   },
    //   {
    //     output: result,
    //   }
    // ); 
    GBLog.info(`GPT Result: ${result.toString()}`);
    return { answer: result.toString(), questionId: 0 };

  }


  private static getToolsAsText(tools) {
    return Object.keys(tools)
      .map((toolname) => `${tools[toolname].name}: ${tools[toolname].description}`)
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
        func.schema = jsonSchemaToZod(func.properties, { module: "esm" });
        func.func = async () => {
          const name = '';
          const pid = 1;
          const text = ''; // TODO:
          const result = await GBVMService.callVM(name, min, false, pid, false, [text]);

        }

        functions.push(func);
      }

    });

    const multiplyTool = new DynamicStructuredTool({
      name: "multiply",
      description: "Multiply two integers together.",
      schema: z.object({
        firstInt: z.number(),
        secondInt: z.number(),
      }),
      func: async ({ firstInt, secondInt }) => {
        return (firstInt * secondInt).toString();
      },
    });

    functions.push(multiplyTool);


    return functions;
  }
}
