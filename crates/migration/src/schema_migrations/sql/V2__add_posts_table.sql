-- Add posts table and related tables

-- Create posts table
CREATE TABLE IF NOT EXISTS posts (
    id UUID PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_published BOOLEAN NOT NULL DEFAULT false,
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create comments table
CREATE TABLE IF NOT EXISTS comments (
    id UUID PRIMARY KEY,
    content TEXT NOT NULL,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create post_likes table for many-to-many relationship
CREATE TABLE IF NOT EXISTS post_likes (
    id UUID PRIMARY KEY,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(post_id, user_id)
);

-- Enable RLS on new tables
ALTER TABLE posts ENABLE ROW LEVEL SECURITY;
ALTER TABLE comments ENABLE ROW LEVEL SECURITY;
ALTER TABLE post_likes ENABLE ROW LEVEL SECURITY;

-- Create RLS policies for posts
CREATE POLICY "Users can view all published posts" 
ON posts FOR SELECT 
USING (is_published = true);

CREATE POLICY "Users can view their own unpublished posts" 
ON posts FOR SELECT 
TO authenticated
USING (user_id = auth.uid());

CREATE POLICY "Users can create posts" 
ON posts FOR INSERT 
TO authenticated
WITH CHECK (user_id = auth.uid());

CREATE POLICY "Users can update their own posts" 
ON posts FOR UPDATE 
TO authenticated
USING (user_id = auth.uid())
WITH CHECK (user_id = auth.uid());

CREATE POLICY "Users can delete their own posts" 
ON posts FOR DELETE 
TO authenticated
USING (user_id = auth.uid());

-- Create RLS policies for comments
CREATE POLICY "Users can view comments on posts they can view" 
ON comments FOR SELECT 
USING (
    post_id IN (
        SELECT id FROM posts WHERE is_published = true 
        UNION 
        SELECT id FROM posts WHERE user_id = auth.uid()
    )
);

CREATE POLICY "Users can create comments on posts they can view" 
ON comments FOR INSERT 
TO authenticated
WITH CHECK (
    user_id = auth.uid() AND
    post_id IN (
        SELECT id FROM posts WHERE is_published = true 
        UNION 
        SELECT id FROM posts WHERE user_id = auth.uid()
    )
);

CREATE POLICY "Users can update their own comments" 
ON comments FOR UPDATE 
TO authenticated
USING (user_id = auth.uid())
WITH CHECK (user_id = auth.uid());

CREATE POLICY "Users can delete their own comments" 
ON comments FOR DELETE 
TO authenticated
USING (user_id = auth.uid());

-- Create RLS policies for post_likes
CREATE POLICY "Users can view likes on posts they can view" 
ON post_likes FOR SELECT 
USING (
    post_id IN (
        SELECT id FROM posts WHERE is_published = true 
        UNION 
        SELECT id FROM posts WHERE user_id = auth.uid()
    )
);

CREATE POLICY "Users can like posts they can view" 
ON post_likes FOR INSERT 
TO authenticated
WITH CHECK (
    user_id = auth.uid() AND
    post_id IN (
        SELECT id FROM posts WHERE is_published = true 
        UNION 
        SELECT id FROM posts WHERE user_id = auth.uid()
    )
);

CREATE POLICY "Users can remove their own likes" 
ON post_likes FOR DELETE 
TO authenticated
USING (user_id = auth.uid()); 