import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "灵犀节点 SparkNode · 高维叙事引擎",
  description:
    "高维叙事与世界演算引擎 — 从文本生成到4D世界模型实时演算",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="zh-CN">
      <body className="min-h-screen antialiased">{children}</body>
    </html>
  );
}