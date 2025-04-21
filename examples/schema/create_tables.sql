-- tasksテーブルの作成
CREATE TABLE IF NOT EXISTS public.tasks (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    is_complete BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    user_id UUID NOT NULL
);

-- RLSポリシーの設定
ALTER TABLE public.tasks ENABLE ROW LEVEL SECURITY;

-- ユーザー自身のタスクの読み取りを許可
CREATE POLICY "ユーザー自身のタスクの読み取りを許可" ON public.tasks 
FOR SELECT USING (auth.uid() = user_id);

-- ユーザー自身のタスクの作成を許可
CREATE POLICY "ユーザー自身のタスクの作成を許可" ON public.tasks 
FOR INSERT WITH CHECK (auth.uid() = user_id);

-- ユーザー自身のタスクの更新を許可
CREATE POLICY "ユーザー自身のタスクの更新を許可" ON public.tasks 
FOR UPDATE USING (auth.uid() = user_id);

-- ユーザー自身のタスクの削除を許可
CREATE POLICY "ユーザー自身のタスクの削除を許可" ON public.tasks 
FOR DELETE USING (auth.uid() = user_id);

-- 匿名ユーザーにtasksテーブルへのアクセス権を付与
GRANT SELECT, INSERT, UPDATE, DELETE ON public.tasks TO anon;
GRANT USAGE, SELECT ON SEQUENCE public.tasks_id_seq TO anon;

-- 認証済みユーザーにtasksテーブルへのアクセス権を付与
GRANT SELECT, INSERT, UPDATE, DELETE ON public.tasks TO authenticated;
GRANT USAGE, SELECT ON SEQUENCE public.tasks_id_seq TO authenticated;

-- サービスロールにtasksテーブルへのアクセス権を付与
GRANT SELECT, INSERT, UPDATE, DELETE ON public.tasks TO service_role;
GRANT USAGE, SELECT ON SEQUENCE public.tasks_id_seq TO service_role; 