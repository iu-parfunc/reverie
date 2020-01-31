#include <sys/types.h>
#include <pthread.h>
#include <time.h>
#include <stdlib.h>
#include <stdio.h>

#ifndef ARRAY_SIZE
#define ARRAY_SIZE(x)    ( (sizeof(x)) / sizeof((x)[0]) )
#endif

#define NR_THREADS 5

static pthread_cond_t conds[NR_THREADS] = {
  PTHREAD_COND_INITIALIZER,
  PTHREAD_COND_INITIALIZER,
  PTHREAD_COND_INITIALIZER,
  PTHREAD_COND_INITIALIZER,
  PTHREAD_COND_INITIALIZER,
};

static pthread_mutex_t mutex = PTHREAD_MUTEX_INITIALIZER;

void* thread_entry(void* param)
{
  long id = (long)param;

  printf("this is thread #%lu\n", id);

  pthread_mutex_lock(&mutex);

  pthread_cond_wait(&conds[id], &mutex);
  pthread_cond_signal(&conds[(1+id) % NR_THREADS]);

  pthread_mutex_unlock(&mutex);

  printf("%lu exited.\n", id);

  return 0;
}

int main(int argc, char* argv[])
{
  pthread_t ids[NR_THREADS];
  struct timespec tp = {0, 100000000};

  for (long i = 0; i < NR_THREADS; i++) {
    pthread_create(&ids[i], NULL, thread_entry, (void*)i);
  }

  nanosleep(&tp, NULL);

  int k = 3;
  printf("signaling thread #%u\n", k);
  pthread_cond_signal(&conds[k]);

  for (int i = 0; i < NR_THREADS; i++) {
    pthread_join(ids[i], NULL);
  }

  for (int i = 0; i < NR_THREADS; i++) {
    pthread_cond_destroy(&conds[i]);
  }

  return 0;
}
