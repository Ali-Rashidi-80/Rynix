/* Suite12 #2 — binary trees depth 16 (End suite12_c.c). Locked checksum across langs. */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct TreeNode {
  struct TreeNode *left;
  struct TreeNode *right;
  int32_t val;
} TreeNode;

static TreeNode *create_tree(int depth) {
  TreeNode *node = (TreeNode *)malloc(sizeof(TreeNode));
  if (!node) {
    return NULL;
  }
  node->val = depth;
  if (depth > 0) {
    node->left = create_tree(depth - 1);
    node->right = create_tree(depth - 1);
  } else {
    node->left = NULL;
    node->right = NULL;
  }
  return node;
}

static int64_t sum_tree(TreeNode *node) {
  if (!node) {
    return 0;
  }
  return node->val + sum_tree(node->left) - sum_tree(node->right);
}

static void free_tree(TreeNode *node) {
  if (!node) {
    return;
  }
  free_tree(node->left);
  free_tree(node->right);
  free(node);
}

int64_t bench_2_binary_trees(void) {
  int max_depth = 16;
  TreeNode *stretch = create_tree(max_depth + 1);
  int64_t check = sum_tree(stretch);
  free_tree(stretch);

  TreeNode *long_lived = create_tree(max_depth);
  for (int depth = 4; depth <= max_depth; depth += 2) {
    int iterations = 1 << (max_depth - depth + 4);
    for (int i = 1; i <= iterations; i++) {
      TreeNode *t1 = create_tree(depth);
      check += sum_tree(t1);
      free_tree(t1);
    }
  }
  check += sum_tree(long_lived);
  free_tree(long_lived);
  return check;
}

int main(void) {
  printf("checksum=%lld\n", (long long)bench_2_binary_trees());
  return 0;
}
