#include <sys/types.h>
#include <time.h>
#include <stdlib.h>
#include <stdio.h>
#include <errno.h>
#include <string.h>

#define NITERATIONS 1000

static long long diff_time(const struct timespec* begin,
         const struct timespec* end) {
  long long r = 0;
  r = (end->tv_sec - begin->tv_sec) * 1000000000 + (end->tv_nsec - begin->tv_nsec);
  return r / 1000;
}

int main(int argc, char* argv[])
{
  struct timespec req = {
    .tv_sec = 0,
    .tv_nsec = 1000000,
  };
  struct timespec begin, end;
  int ntests = NITERATIONS;

  // ignore first nanosleep
  nanosleep(&req, NULL);

  // ignore first clock_gettime
  clock_gettime(CLOCK_REALTIME, &end);

  clock_gettime(CLOCK_REALTIME, &begin);

  for (int i = 0; i < 1000; i++) {
    printf("nanosleep, iteration: %u\n", i);
    nanosleep(&req, NULL);
  }
  clock_gettime(CLOCK_REALTIME, &end);

  long long elapsed = diff_time(&begin, &end);

  printf("time elapsed %lluus for %u iterations, mean: %.3lfus\n",
   elapsed, ntests, (double)elapsed / ntests);

  return 0;
}
