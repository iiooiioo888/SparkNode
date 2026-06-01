"""
灵犀节点 LLM 路由调度服务

基于 FastAPI 的多模型并行生成服务。
支持 OpenAI、Anthropic、本地模型的统一调度。
"""

import asyncio
import os
import time
from typing import AsyncIterator

from dotenv import load_dotenv
from fastapi import FastAPI, Request
from fastapi.responses import StreamingResponse
from pydantic import BaseModel

load_dotenv()

app = FastAPI(
    title="SparkNode LLM Router",
    description="灵犀节点 · 多模型并行流式生成服务",
    version="0.1.0",
)


# ── 请求/响应模型 ──────────────────────────────────

class GenerateRequest(BaseModel):
    story_id: str
    node_id: str | None = None
    prompt: str
    provider: str = "openai"
    temperature: float = 0.7
    max_tokens: int = 2048


class CompareRequest(BaseModel):
    story_id: str
    prompt: str
    providers: list[str] = ["openai", "anthropic"]
    temperature: float = 0.7
    max_tokens: int = 2048


# ── LLM Provider 抽象 ─────────────────────────────

class LLMProvider:
    """LLM 提供商基类"""

    def __init__(self, name: str):
        self.name = name

    async def generate_stream(self, prompt: str, temperature: float, max_tokens: int) -> AsyncIterator[str]:
        raise NotImplementedError


class OpenAIProvider(LLMProvider):
    def __init__(self):
        super().__init__("openai")
        self.api_key = os.getenv("OPENAI_API_KEY", "")
        self.model = os.getenv("OPENAI_MODEL", "gpt-4o")

    async def generate_stream(self, prompt: str, temperature: float, max_tokens: int) -> AsyncIterator[str]:
        from openai import AsyncOpenAI
        client = AsyncOpenAI(api_key=self.api_key)
        stream = await client.chat.completions.create(
            model=self.model,
            messages=[{"role": "user", "content": prompt}],
            temperature=temperature,
            max_tokens=max_tokens,
            stream=True,
        )
        async for chunk in stream:
            if chunk.choices[0].delta.content:
                yield chunk.choices[0].delta.content


class AnthropicProvider(LLMProvider):
    def __init__(self):
        super().__init__("anthropic")
        self.api_key = os.getenv("ANTHROPIC_API_KEY", "")
        self.model = os.getenv("ANTHROPIC_MODEL", "claude-sonnet-4-20250514")

    async def generate_stream(self, prompt: str, temperature: float, max_tokens: int) -> AsyncIterator[str]:
        import anthropic
        client = anthropic.AsyncAnthropic(api_key=self.api_key)
        async with client.messages.stream(
            model=self.model,
            max_tokens=max_tokens,
            temperature=temperature,
            messages=[{"role": "user", "content": prompt}],
        ) as stream:
            async for text in stream.text_stream:
                yield text


class LocalProvider(LLMProvider):
    def __init__(self):
        super().__init__("local")
        self.base_url = os.getenv("LOCAL_LLM_URL", "http://localhost:11434")
        self.model = os.getenv("LOCAL_LLM_MODEL", "llama3")

    async def generate_stream(self, prompt: str, temperature: float, max_tokens: int) -> AsyncIterator[str]:
        import httpx
        async with httpx.AsyncClient() as client:
            async with client.stream(
                "POST",
                f"{self.base_url}/api/generate",
                json={
                    "model": self.model,
                    "prompt": prompt,
                    "stream": True,
                    "options": {"temperature": temperature, "num_predict": max_tokens},
                },
                timeout=120.0,
            ) as response:
                async for line in response.aiter_lines():
                    if line.strip():
                        import json
                        try:
                            data = json.loads(line)
                            if "response" in data:
                                yield data["response"]
                        except json.JSONDecodeError:
                            pass


# ── Provider 注册表 ────────────────────────────────

PROVIDERS: dict[str, LLMProvider] = {
    "openai": OpenAIProvider(),
    "anthropic": AnthropicProvider(),
    "local": LocalProvider(),
}


# ── API 路由 ───────────────────────────────────────

@app.get("/health")
async def health():
    return {
        "status": "online",
        "service": "sp-llm-router",
        "providers": list(PROVIDERS.keys()),
    }


@app.post("/generate/stream")
async def generate_stream(req: GenerateRequest):
    """单模型流式生成 (SSE)"""
    provider = PROVIDERS.get(req.provider)
    if not provider:
        return {"error": f"未知的 provider: {req.provider}"}

    async def event_generator():
        async for chunk in provider.generate_stream(req.prompt, req.temperature, req.max_tokens):
            yield f"data: {chunk}\n\n"
        yield "data: [DONE]\n\n"

    return StreamingResponse(
        event_generator(),
        media_type="text/event-stream",
        headers={"Cache-Control": "no-cache", "X-Provider": req.provider},
    )


@app.post("/generate/compare")
async def generate_compare(req: CompareRequest):
    """多模型并行对比生成 (灵犀矩阵)"""
    results = []

    async def fetch_one(provider_name: str):
        provider = PROVIDERS.get(provider_name)
        if not provider:
            return {"provider": provider_name, "error": "未知的 provider"}

        start = time.time()
        chunks = []
        async for chunk in provider.generate_stream(req.prompt, req.temperature, req.max_tokens):
            chunks.append(chunk)
        latency = (time.time() - start) * 1000

        return {
            "provider": provider_name,
            "content": "".join(chunks),
            "latency_ms": round(latency, 2),
            "tokens_used": 0,  # TODO: 从响应头解析
        }

    tasks = [fetch_one(p) for p in req.providers]
    results = await asyncio.gather(*tasks)

    return {
        "story_id": req.story_id,
        "prompt": req.prompt,
        "results": results,
        "provider_count": len(results),
    }


# ── 启动入口 ───────────────────────────────────────

if __name__ == "__main__":
    import uvicorn
    port = int(os.getenv("LLM_ROUTER_PORT", "8001"))
    uvicorn.run(app, host="0.0.0.0", port=port)